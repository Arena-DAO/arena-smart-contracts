use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Reply, Response};
use cw2::set_contract_version;

use crate::{
    execute::{self, TRIGGER_COMPETITION_REPLY_ID},
    msg::{ExecuteMsg, InstantiateMsg},
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
        ExecuteMsg::TriggerCreation { id, escrow_id } => {
            execute::trigger_creation(deps, env, info, id, escrow_id)
        }
        ExecuteMsg::Enroll { id } => execute::enroll(deps, env, info, id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        TRIGGER_COMPETITION_REPLY_ID => {
            Ok(Response::new().add_attribute("reply", "reply_trigger_competition"))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
