#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult};
use cw2::set_contract_version;
use cw_competition::{
    contract::CompetitionModuleContract,
    error::CompetitionError,
    msg::{ExecuteBase, InstantiateBase, QueryBase},
    state::Competition,
};

pub const CONTRACT_NAME: &str = "crates.io:arena-wager-module";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type Wager = Competition<Empty>;
pub type CompetitionModule = CompetitionModuleContract<Empty, Empty, Empty, Empty>;
pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<Empty, Empty>;
pub type QueryMsg = QueryBase<Empty, Empty>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, CompetitionError> {
    let resp = CompetitionModule::default().instantiate(deps.branch(), info, msg)?;
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
