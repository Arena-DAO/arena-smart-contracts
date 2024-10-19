use crate::{execute, query, state::MEMBER_COUNT, ContractError};
use arena_interface::group::{AddMemberMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint64, WasmMsg,
};
use cw2::{ensure_from_older_version, set_contract_version};

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-group";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let msgs = instantiate_contract(deps, &env, &info, msg.members)?;

    Ok(Response::default().add_messages(msgs))
}

pub fn instantiate_contract(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    members: Option<Vec<AddMemberMsg>>,
) -> Result<Vec<CosmosMsg>, ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    MEMBER_COUNT.save(deps.storage, &Uint64::zero())?;

    Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::UpdateMembers {
            to_add: members,
            to_update: None,
            to_remove: None,
        })?,
        funds: vec![],
    })])
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateMembers {
            to_add,
            to_update,
            to_remove,
        } => execute::update_members(deps, env, info, to_add, to_update, to_remove),
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;

            Ok(Response::default().add_attributes(ownership.into_attributes()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Members { start_after, limit } => {
            to_json_binary(&query::members(deps, start_after, limit)?)
        }
        QueryMsg::MembersCount {} => to_json_binary(&MEMBER_COUNT.load(deps.storage)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::IsValidDistribution { addrs } => {
            to_json_binary(&query::is_valid_distribution(deps, addrs)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let _version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
