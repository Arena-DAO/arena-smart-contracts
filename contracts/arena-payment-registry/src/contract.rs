use arena_interface::registry::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_balance::Distribution;

use crate::{execute, query};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-payment-registry";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetDistribution { distribution } => {
            execute::set_distribution(deps, env, info, distribution)
        }
        ExecuteMsg::SetDistributionRemainderSelf { member_percentages } => {
            let distribution = Distribution {
                remainder_addr: info.sender.to_string(),
                member_percentages,
            };

            execute::set_distribution(deps, env, info, distribution)
        }
        ExecuteMsg::RemoveDistribution {} => execute::remove_distribution(deps, env, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDistribution { addr, height } => {
            to_json_binary(&query::get_distribution(deps, env, addr, height)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let _version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
