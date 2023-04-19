use crate::{
    execute,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query,
    state::{ADMIN, DUE, IS_LOCKED, STAKE, TOTAL_BALANCE, TOTAL_STAKE},
    ContractError,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_balance::{BalanceVerified, MemberBalance};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:arena-dao-escrow";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    instantiate_contract(deps, info, msg.dues, msg.stakes)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("addr", env.contract.address))
}

pub fn instantiate_contract(
    mut deps: DepsMut,
    info: MessageInfo,
    due: Vec<MemberBalance>,
    stake: Vec<MemberBalance>,
) -> Result<(), ContractError> {
    ADMIN.set(deps.branch(), Some(info.sender))?;
    IS_LOCKED.save(deps.storage, &false)?;
    let total_stake = BalanceVerified::new();
    for member_balance in stake {
        let member_balance = member_balance.to_verified(deps.as_ref())?;
        STAKE.save(deps.storage, &member_balance.addr, &member_balance.balance)?;
        total_stake.checked_add(&member_balance.balance)?;
    }
    for member_balance in due {
        let member_balance = member_balance.to_verified(deps.as_ref())?;
        let total_due = match STAKE.has(deps.storage, &member_balance.addr) {
            true => STAKE
                .load(deps.storage, &member_balance.addr)?
                .checked_add(&member_balance.balance)?,
            false => member_balance.balance,
        };
        DUE.save(deps.storage, &member_balance.addr, &total_due)?;
    }
    TOTAL_BALANCE.save(deps.storage, &BalanceVerified::new())?;
    TOTAL_STAKE.save(deps.storage, &total_stake)?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
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
        ExecuteMsg::Distribute {
            distribution,
            remainder_addr,
        } => execute::distribute(deps, info, distribution, remainder_addr),
        ExecuteMsg::Lock { value } => execute::lock(deps, info, value),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
        QueryMsg::Balance { addr } => to_binary(&query::balance(deps, addr)?),
        QueryMsg::Due { addr } => to_binary(&query::due(deps, addr)?),
        QueryMsg::TotalBalance {} => to_binary(&query::total_balance(deps)),
        QueryMsg::Stake { addr } => to_binary(&query::stake(deps, addr)?),
        QueryMsg::IsLocked {} => to_binary(&query::is_locked(deps)),
        QueryMsg::DumpState {} => to_binary(&query::dump_state(deps)?),
        QueryMsg::Distribution { addr } => to_binary(&query::distribution(deps, addr)?),
        QueryMsg::IsFunded { addr } => to_binary(&query::is_funded(deps, addr)?),
        QueryMsg::IsFullyFunded {} => to_binary(&query::is_fully_funded(deps)?),
        QueryMsg::DistributableBalance {} => to_binary(&query::distributable_balance(deps)?),
    }
}
