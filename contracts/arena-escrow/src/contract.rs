use crate::{
    execute,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    query,
    state::{self, DUE, INITIAL_DUE, IS_LOCKED},
    ContractError,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_balance::MemberBalance;

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
    if due.is_empty() {
        return Err(ContractError::InvalidDue {
            msg: "None due".to_string(),
        });
    }
    if due.len() == 1 {
        return Err(ContractError::InvalidDue {
            msg: "Only one due".to_string(),
        });
    }

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;
    IS_LOCKED.save(deps.storage, &false)?;
    for member_balance in due {
        let member_balance = member_balance.to_verified(deps.as_ref())?;

        if INITIAL_DUE.has(deps.storage, &member_balance.addr) {
            return Err(ContractError::StdError(
                cosmwasm_std::StdError::GenericErr {
                    msg: "Cannot have duplicate addresses in dues".to_string(),
                },
            ));
        }

        INITIAL_DUE.save(deps.storage, &member_balance.addr, &member_balance.balance)?;
        DUE.save(deps.storage, &member_balance.addr, &member_balance.balance)?;
    }

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
        QueryMsg::Balance { addr } => to_json_binary(&query::balance(deps, addr)?),
        QueryMsg::Due { addr } => to_json_binary(&query::due(deps, addr)?),
        QueryMsg::TotalBalance {} => to_json_binary(&query::total_balance(deps)?),
        QueryMsg::IsLocked {} => to_json_binary(&query::is_locked(deps)),
        QueryMsg::Distribution { addr } => to_json_binary(&query::distribution(deps, addr)?),
        QueryMsg::IsFunded { addr } => to_json_binary(&query::is_funded(deps, addr)?),
        QueryMsg::IsFullyFunded {} => to_json_binary(&state::is_fully_funded(deps)),
        QueryMsg::Balances { start_after, limit } => {
            to_json_binary(&query::balances(deps, start_after, limit)?)
        }
        QueryMsg::Dues { start_after, limit } => {
            to_json_binary(&query::dues(deps, start_after, limit)?)
        }
        QueryMsg::InitialDues { start_after, limit } => {
            to_json_binary(&query::initial_dues(deps, start_after, limit)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::DumpState { addr } => to_json_binary(&query::dump_state(deps, addr)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
