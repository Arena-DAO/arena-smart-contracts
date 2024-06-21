use std::str::FromStr;

use cosmwasm_std::{
    entry_point, Attribute, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
    SubMsgResult, Uint128,
};
use cw2::set_contract_version;

use crate::{
    execute::{self, TRIGGER_COMPETITION_REPLY_ID},
    msg::{ExecuteMsg, InstantiateMsg},
    state::{enrollment_entries, CompetitionInfo, TEMP_ENROLLMENT_INFO},
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-competition-enrollment";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let ownership =
        cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

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
            is_creator_member,
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
            is_creator_member,
        ),
        ExecuteMsg::TriggerExpiration { id, escrow_id } => {
            execute::trigger_expiration(deps, env, info, id, escrow_id)
        }
        ExecuteMsg::Enroll { id } => execute::enroll(deps, env, info, id),
        ExecuteMsg::Withdraw { id } => todo!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        TRIGGER_COMPETITION_REPLY_ID => {
            let (module_addr, enrollment_id) = TEMP_ENROLLMENT_INFO.load(deps.storage)?;
            let attrs = match msg.result {
                SubMsgResult::Ok(response) => {
                    let competition_id = response
                        .events
                        .iter()
                        .find(|x| {
                            x.attributes
                                .iter()
                                .find(|y| y.key == "action" && y.value == "create_competition")
                                .is_some()
                        })
                        .map(|x| {
                            x.attributes
                                .iter()
                                .find(|y| y.key == "competition_id")
                                .map(|y| y.value.clone())
                        })
                        .flatten();

                    if let Some(competition_id) = competition_id {
                        enrollment_entries().update(
                            deps.storage,
                            enrollment_id,
                            |x| -> StdResult<_> {
                                match x {
                                    Some(mut enrollment_entry) => {
                                        enrollment_entry.has_triggered_expiration = true;
                                        enrollment_entry.competition_info =
                                            CompetitionInfo::Existing {
                                                module_addr,
                                                id: Uint128::from_str(&competition_id)?,
                                            };

                                        Ok(enrollment_entry)
                                    }
                                    None => Err(StdError::generic_err(format!(
                                        "Cannot find the enrollment entry {}",
                                        enrollment_id
                                    ))),
                                }
                            },
                        )?;

                        vec![]
                    } else {
                        return Err(ContractError::StdError(StdError::generic_err(
                            "Cannot determine the competition id",
                        )));
                    }
                }
                SubMsgResult::Err(error_message) => {
                    enrollment_entries().update(
                        deps.storage,
                        enrollment_id,
                        |x| -> StdResult<_> {
                            match x {
                                Some(mut enrollment_entry) => {
                                    enrollment_entry.has_triggered_expiration = true;

                                    Ok(enrollment_entry)
                                }
                                None => Err(StdError::generic_err(format!(
                                    "Cannot find the enrollment entry {}",
                                    enrollment_id
                                ))),
                            }
                        },
                    )?;

                    vec![Attribute::new("error", error_message)]
                }
            };

            Ok(Response::new()
                .add_attribute("reply", "reply_trigger_competition")
                .add_attributes(attrs))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
