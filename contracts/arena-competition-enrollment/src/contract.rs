use std::str::FromStr;

use arena_interface::{
    competition::types::{CompetitionType, EliminationType},
    enrollments::QueryMsg,
    escrow::{self, TransferEscrowOwnershipMsg},
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, SubMsgResult, Uint128, WasmMsg,
};
use cw2::{ensure_from_older_version, set_contract_version};

use crate::{
    execute::{self, FINALIZE_COMPETITION_REPLY_ID},
    migrate,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg},
    query,
    state::{enrollment_entries, CompetitionInfo, ENROLLMENT_COUNT, TEMP_ENROLLMENT_INFO},
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-competition-enrollment";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ENROLLMENT_COUNT.save(deps.storage, &Uint128::zero())?;
    let owner = deps.api.addr_validate(&msg.owner)?;
    let ownership = cw_ownable::initialize_owner(deps.storage, deps.api, Some(owner.as_str()))?;

    Ok(Response::new().add_attributes(ownership.into_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
        ExecuteMsg::CreateEnrollment {
            min_members,
            max_members,
            entry_fee,
            duration_before,
            category_id,
            competition_info,
            competition_type,
            group_contract_info,
            required_team_size,
            escrow_contract_info,
            use_dao_host,
        } => execute::create_enrollment(
            deps,
            env,
            info,
            min_members,
            max_members,
            entry_fee,
            duration_before,
            category_id,
            competition_info,
            competition_type,
            group_contract_info,
            required_team_size,
            escrow_contract_info,
            use_dao_host,
        ),
        ExecuteMsg::SetRankings { id, rankings } => {
            execute::set_rankings(deps, env, info, id, rankings)
        }
        ExecuteMsg::Finalize { id } => execute::finalize(deps, env, info, id),
        ExecuteMsg::Enroll { id, team } => execute::enroll(deps, env, info, id, team),
        ExecuteMsg::Withdraw { id, team } => execute::withdraw(deps, env, info, id, team),
        ExecuteMsg::ForceWithdraw { id, members } => {
            execute::force_withdraw(deps, env, info, id, members)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Enrollments {
            start_after,
            limit,
            filter,
        } => to_json_binary(&query::enrollments(deps, start_after, limit, filter)?),
        QueryMsg::Enrollment { enrollment_id } => {
            let entry = enrollment_entries().load(deps.storage, enrollment_id.u128())?;
            to_json_binary(&entry.into_response(deps, enrollment_id)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::EnrollmentCount {} => to_json_binary(&query::enrollment_count(deps)?),
        QueryMsg::IsMember {
            addr,
            enrollment_id,
        } => to_json_binary(&query::is_member(deps, enrollment_id, addr)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let _version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg {
        MigrateMsg::FromCompatible {} => {}
        MigrateMsg::RemoveThirdPlaceMatch { enrollment_id } => {
            enrollment_entries().update(
                deps.storage,
                enrollment_id.u128(),
                |x| -> StdResult<_> {
                    if let Some(mut enrollment) = x {
                        match &mut enrollment.competition_type {
                            CompetitionType::Tournament {
                                elimination_type,
                                distribution: _,
                            } if matches!(
                                elimination_type,
                                EliminationType::SingleElimination { .. }
                            ) =>
                            {
                                *elimination_type = EliminationType::SingleElimination {
                                    play_third_place_match: false,
                                };
                            }
                            _ => {}
                        };

                        Ok(enrollment)
                    } else {
                        Err(StdError::generic_err("Enrollment not found"))
                    }
                },
            )?;
        }
        MigrateMsg::FromV2_3 {} => {
            migrate::migrate_from_v2_3_to_v2_3_1(deps.branch(), env)?;
        }
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        FINALIZE_COMPETITION_REPLY_ID => {
            let enrollment_info = TEMP_ENROLLMENT_INFO.load(deps.storage)?;
            match msg.result {
                SubMsgResult::Ok(response) => {
                    let event = response.events.iter().find(|x| {
                        x.attributes
                            .iter()
                            .any(|y| y.key == "action" && y.value == "create_competition")
                    });

                    let competition_id = event.and_then(|x| {
                        x.attributes
                            .iter()
                            .find(|y| y.key == "competition_id")
                            .map(|y| y.value.clone())
                    });

                    if let Some(competition_id) = competition_id {
                        enrollment_entries().update(
                            deps.storage,
                            enrollment_info.enrollment_id,
                            |x| -> StdResult<_> {
                                match x {
                                    Some(mut enrollment_entry) => {
                                        enrollment_entry.competition_info =
                                            CompetitionInfo::Existing {
                                                id: Uint128::from_str(&competition_id)?,
                                            };
                                        Ok(enrollment_entry)
                                    }
                                    None => Err(StdError::generic_err(format!(
                                        "Cannot find the enrollment entry {}",
                                        enrollment_info.enrollment_id
                                    ))),
                                }
                            },
                        )?;

                        let escrow_msg = WasmMsg::Execute {
                            contract_addr: enrollment_info.escrow_addr.to_string(),
                            msg: to_json_binary(&escrow::ExecuteMsg::Lock {
                                value: true,
                                transfer_ownership: Some(TransferEscrowOwnershipMsg {
                                    addr: enrollment_info.module_addr.to_string(),
                                    is_enrollment: false,
                                }),
                            })?,
                            funds: vec![],
                        };

                        Ok(Response::new()
                            .add_attribute("reply", "reply_finalize")
                            .add_attribute("result", "competition_created")
                            .add_message(escrow_msg))
                    } else {
                        Err(ContractError::StdError(StdError::generic_err(
                            "Missing competition_id",
                        )))
                    }
                }
                SubMsgResult::Err(error_message) => {
                    let escrow_msg = WasmMsg::Execute {
                        contract_addr: enrollment_info.escrow_addr.to_string(),
                        msg: to_json_binary(&escrow::ExecuteMsg::Lock {
                            value: false,
                            transfer_ownership: None,
                        })?,
                        funds: vec![],
                    };

                    Ok(Response::new()
                        .add_attribute("reply", "reply_finalize")
                        .add_attribute("error", error_message)
                        .add_message(escrow_msg))
                }
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
