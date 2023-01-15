use std::collections::HashMap;

use crate::{
    execute,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query,
    state::{ARBITER, DUE, KEY, STAKE, TOTAL_BALANCE},
    ContractError,
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;
use cw_disbursement::MemberBalance;
use cw_tokens::{GenericBalanceExtensions, GenericTokenBalance};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:agon-wager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let arbiter = match msg.arbiter {
        Some(x) => deps.api.addr_validate(&x)?,
        None => info.sender,
    };
    instantiate_contract(deps, arbiter, msg.due, msg.stake)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("addr", env.contract.address))
}

pub fn instantiate_contract(
    deps: DepsMut,
    arbiter: Addr,
    due: Vec<MemberBalance>,
    stake: Vec<MemberBalance>,
) -> Result<(), ContractError> {
    ARBITER.save(deps.storage, &arbiter)?;
    let mut stake_set: HashMap<Addr, Vec<GenericTokenBalance>> = HashMap::new();
    for member_balance in stake {
        let addr = deps.api.addr_validate(&member_balance.member)?;
        if stake_set.contains_key(&addr) {
            return Err(ContractError::StdError(StdError::generic_err(
                "Invalid stake set".to_string(),
            )));
        }
        STAKE.save(deps.storage, addr.clone(), &member_balance.balances)?;
        stake_set.insert(addr, member_balance.balances);
    }
    for mut member_balance in due {
        let addr = deps.api.addr_validate(&member_balance.member)?;
        let to_add = stake_set.remove(&addr);
        if to_add.is_some() {
            member_balance.balances = member_balance
                .balances
                .add_balances_checked(&to_add.unwrap())?;
        }
        DUE.save(deps.storage, addr, &member_balance.balances)?;
    }
    for member_balance in stake_set {
        DUE.save(deps.storage, member_balance.0, &member_balance.1)?;
    }
    TOTAL_BALANCE.save(deps.storage, &vec![])?;

    KEY.save(deps.storage, &arbiter)?;
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
        ExecuteMsg::Refund {} => execute::refund(deps.storage, deps.querier, &vec![info.sender]),
        ExecuteMsg::Receive(cw20_receive_msg) => {
            execute::cw20_receive(deps, info, cw20_receive_msg)
        }
        ExecuteMsg::ReceiveNft(cw721_receive_msg) => {
            execute::cw721_receive(deps, info, cw721_receive_msg)
        }
        ExecuteMsg::HandleCompetitionResult(cw_competition_result_msg) => {
            execute::handle_competition_result(deps, info, cw_competition_result_msg)
        }
        ExecuteMsg::HandleCompetitionStateChanged(cw_competition_state_changed_msg) => {
            execute::handle_competition_state_changed(deps, info, cw_competition_state_changed_msg)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { member } => to_binary(&query::balance(deps, member)?),
        QueryMsg::Due { member } => to_binary(&query::due(deps, member)?),
        QueryMsg::Total {} => to_binary(&query::total(deps)?),
    }
}
