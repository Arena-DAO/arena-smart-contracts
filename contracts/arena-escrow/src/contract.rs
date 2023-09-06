use crate::{
    execute,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    query,
    state::{DUE, IS_FUNDED, IS_LOCKED, TOTAL_BALANCE},
    ContractError,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_balance::{BalanceVerified, MemberBalance};

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-escrow";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    instantiate_contract(deps, info, msg.dues)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("addr", env.contract.address))
}

pub fn instantiate_contract(
    deps: DepsMut,
    info: MessageInfo,
    due: Vec<MemberBalance>,
) -> Result<(), ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;
    IS_LOCKED.save(deps.storage, &false)?;
    for member_balance in due {
        let member_balance = member_balance.to_verified(deps.as_ref())?;
        DUE.save(deps.storage, &member_balance.addr, &member_balance.balance)?;
        IS_FUNDED.save(deps.storage, &member_balance.addr, &false)?;
    }
    TOTAL_BALANCE.save(deps.storage, &BalanceVerified::new())?;

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNative {} => execute::receive_native(deps, info),
        ExecuteMsg::Withdraw {
            cw20_msg,
            cw721_msg,
        } => execute::withdraw(deps, info, cw20_msg, cw721_msg),
        ExecuteMsg::SetDistribution { distribution } => {
            execute::set_distribution(deps, info, distribution)
        }
        ExecuteMsg::Receive(cw20_receive_msg) => {
            execute::receive_cw20(deps, info, cw20_receive_msg)
        }
        ExecuteMsg::ReceiveNft(cw721_receive_msg) => {
            execute::receive_cw721(deps, info, cw721_receive_msg)
        }
        ExecuteMsg::Distribute(competition_escrow_distribute_msg) => execute::distribute(
            deps,
            info,
            competition_escrow_distribute_msg.distribution,
            competition_escrow_distribute_msg.remainder_addr,
        ),
        ExecuteMsg::Lock { value } => execute::lock(deps, info, value),
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { addr } => to_binary(&query::balance(deps, addr)?),
        QueryMsg::Due { addr } => to_binary(&query::due(deps, addr)?),
        QueryMsg::TotalBalance {} => to_binary(&query::total_balance(deps)),
        QueryMsg::IsLocked {} => to_binary(&query::is_locked(deps)),
        QueryMsg::Distribution { addr } => to_binary(&query::distribution(deps, addr)?),
        QueryMsg::IsFunded { addr } => to_binary(&query::is_funded(deps, addr)?),
        QueryMsg::IsFullyFunded {} => to_binary(&query::is_fully_funded(deps)?),
        QueryMsg::Balances { start_after, limit } => {
            to_binary(&query::balances(deps, start_after, limit)?)
        }
        QueryMsg::Dues { start_after, limit } => to_binary(&query::dues(deps, start_after, limit)?),
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
