use crate::{
    execute::{self},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    query, ContractError,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw4_group::contract::{
    create, execute_update_members, query_list_members, query_member, query_total_weight,
};
use cw4_group::{
    msg::ExecuteMsg as Cw4GroupExecuteMsg,
    state::{ADMIN, HOOKS},
};

use cw_disbursement::CwDisbursementExecuteMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw4-team";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    create(
        deps.branch(),
        Some(info.sender.to_string()),
        msg.members,
        env.block.height,
    )?;
    Ok(Response::default().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Cw4GroupExecute(cw4_group_execute_msg) => match cw4_group_execute_msg {
            Cw4GroupExecuteMsg::UpdateAdmin { admin } => execute::update_admin(deps, info, admin),
            Cw4GroupExecuteMsg::AddHook { addr } => {
                let addr = deps.api.addr_validate(&addr)?;
                Ok(HOOKS.execute_add_hook(&ADMIN, deps, info, addr)?)
            }
            Cw4GroupExecuteMsg::RemoveHook { addr } => {
                let addr = deps.api.addr_validate(&addr)?;
                Ok(HOOKS.execute_remove_hook(&ADMIN, deps, info, addr)?)
            }
            Cw4GroupExecuteMsg::UpdateMembers { remove, add } => {
                Ok(execute_update_members(deps, env, info, add, remove)?)
            }
        },
        ExecuteMsg::CwDisbursementExecute(cw_disbursement_execute_msg) => {
            match cw_disbursement_execute_msg {
                CwDisbursementExecuteMsg::SetDisbursementData {
                    key,
                    disbursement_data,
                } => execute::set_disbursement_data(deps, info, key, disbursement_data),
                CwDisbursementExecuteMsg::ReceiveNative { key } => {
                    execute::receive_native(deps, info, key)
                }
            }
        }
        ExecuteMsg::Receive(cw20_receive_msg) => {
            execute::cw20_receive(deps, info, cw20_receive_msg)
        }
        ExecuteMsg::ReceiveNft(cw721_receive_msg) => {
            execute::cw721_receive(deps, info, cw721_receive_msg)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
        QueryMsg::TotalWeight { at_height } => to_binary(&query_total_weight(deps, at_height)?),
        QueryMsg::ListMembers { start_after, limit } => {
            to_binary(&query_list_members(deps, start_after, limit)?)
        }
        QueryMsg::Member { addr, at_height } => to_binary(&query_member(deps, addr, at_height)?),
        QueryMsg::Hooks {} => to_binary(&HOOKS.query_hooks(deps)?),
        QueryMsg::DumpState {} => to_binary(&query::dump_state(deps)?),
        QueryMsg::TotalPowerAtHeight { height } => {
            to_binary(&query::total_weight_at_height(deps, env, height)?)
        }
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_binary(&query::voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::LastUpdated {} => to_binary(&query::last_updated(deps)?),
        QueryMsg::DisbursementData { key } => to_binary(&query::disbursement_data(deps, key)?),
        QueryMsg::Info {} => to_binary(&query::info(deps)?),
        QueryMsg::Dao {} => to_binary(&query::dao(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Don't do any state migrations.
    Ok(Response::default())
}
