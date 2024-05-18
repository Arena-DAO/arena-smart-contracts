#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult};
use cw2::set_contract_version;
use cw_competition_base::{contract::CompetitionModuleContract, error::CompetitionError};

use crate::msg::{EmptyWrapper, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-wager-module";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type CompetitionModule = CompetitionModuleContract<Empty, Empty, Empty, Empty, EmptyWrapper>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, CompetitionError> {
    let resp = CompetitionModule::default().instantiate(deps.branch(), env, info, msg)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, CompetitionError> {
    CompetitionModule::default().execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, CompetitionError> {
    CompetitionModule::default().reply(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    CompetitionModule::default().query(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, CompetitionError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
