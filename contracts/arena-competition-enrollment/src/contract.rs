use std::str::FromStr;

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsgResult, Uint128, WasmMsg,
};
use cw2::{ensure_from_older_version, set_contract_version};

use crate::{
    execute::{self, TRIGGER_COMPETITION_REPLY_ID},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
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
            expiration,
            category_id,
            competition_info,
            competition_type,
            group_contract_info,
        } => execute::create_enrollment(
            deps,
            env,
            info,
            min_members,
            max_members,
            entry_fee,
            expiration,
            category_id,
            competition_info,
            competition_type,
            group_contract_info,
        ),
        ExecuteMsg::TriggerExpiration { id, escrow_id } => {
            execute::trigger_expiration(deps, env, info, id, escrow_id)
        }
        ExecuteMsg::Enroll { id } => execute::enroll(deps, env, info, id),
        ExecuteMsg::Withdraw { id } => execute::withdraw(deps, env, info, id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Enrollments {
            start_after,
            limit,
            filter,
        } => to_json_binary(&query::enrollments(deps, env, start_after, limit, filter)?),
        QueryMsg::Enrollment { enrollment_id } => {
            let entry = enrollment_entries().load(deps.storage, enrollment_id.u128())?;
            to_json_binary(&entry.into_response(deps, &env.block, enrollment_id)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::EnrollmentCount {} => to_json_binary(&query::enrollment_count(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let _version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        TRIGGER_COMPETITION_REPLY_ID => {
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

                    let escrow_addr = event.and_then(|x| {
                        x.attributes
                            .iter()
                            .find(|y| y.key == "escrow_addr")
                            .map(|y| y.value.clone())
                    });

                    if let Some(competition_id) = competition_id {
                        let mut msgs = vec![];
                        enrollment_entries().update(
                            deps.storage,
                            enrollment_info.enrollment_id,
                            |x| -> StdResult<_> {
                                match x {
                                    Some(mut enrollment_entry) => {
                                        enrollment_entry.has_triggered_expiration = true;
                                        enrollment_entry.competition_info =
                                            CompetitionInfo::Existing {
                                                id: Uint128::from_str(&competition_id)?,
                                            };
                                        if let Some(escrow_addr) = escrow_addr {
                                            if let Ok(escrow_addr) =  deps.api.addr_validate(&escrow_addr){
                                                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                                                    contract_addr: escrow_addr.to_string(),
                                                    msg: to_json_binary(&arena_interface::escrow::ExecuteMsg::ReceiveNative {
                                                 })?,
                                                 funds: vec![enrollment_info.amount.unwrap()] }));
                                            }
                                        }
                                        Ok(enrollment_entry)
                                    }
                                    None => Err(StdError::generic_err(format!(
                                        "Cannot find the enrollment entry {}",
                                        enrollment_info.enrollment_id
                                    ))),
                                }
                            },
                        )?;
                        Ok(Response::new()
                            .add_attribute("reply", "reply_trigger_competition")
                            .add_attribute("result", "competition_created")
                            .add_messages(msgs))
                    } else {
                        Err(ContractError::StdError(StdError::generic_err(
                            "Missing competition_id",
                        )))
                    }
                }
                SubMsgResult::Err(error_message) => {
                    enrollment_entries().update(
                        deps.storage,
                        enrollment_info.enrollment_id,
                        |x| -> StdResult<_> {
                            match x {
                                Some(mut enrollment_entry) => {
                                    enrollment_entry.has_triggered_expiration = true;

                                    Ok(enrollment_entry)
                                }
                                None => Err(StdError::generic_err(format!(
                                    "Cannot find the enrollment entry {}",
                                    enrollment_info.enrollment_id
                                ))),
                            }
                        },
                    )?;

                    Ok(Response::new()
                        .add_attribute("reply", "reply_trigger_competition")
                        .add_attribute("error", error_message))
                }
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
