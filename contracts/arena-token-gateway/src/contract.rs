#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};
use cw2::set_contract_version;

use crate::{
    execute,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query,
    state::VESTING_CONFIGURATION,
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-token-gateway";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    Ok(
        Response::default().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::UpdateVestingConfiguration { config: msg.config })?,
            funds: vec![],
        })),
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateVestingConfiguration { config } => {
            execute::update_vesting_configuration(deps, env, info, config)
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::default().add_attributes(ownership.into_attributes()))
        }
        ExecuteMsg::Apply(msg) => execute::apply(deps, env, info, msg),
        ExecuteMsg::AcceptApplication { applicant } => todo!(),
        ExecuteMsg::RejectApplication { applicant, reason } => todo!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Application { applicant } => {
            to_json_binary(&query::application(deps, applicant)?)
        }
        QueryMsg::Applications {
            start_after,
            limit,
            status,
        } => to_json_binary(&query::list_applications(deps, start_after, limit, status)?),
        QueryMsg::VestingConfiguration {} => {
            to_json_binary(&VESTING_CONFIGURATION.load(deps.storage)?)
        }
    }
}
