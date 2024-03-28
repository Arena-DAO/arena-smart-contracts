#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, BlockInfo, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Timestamp, Uint128,
};
use cw2::set_contract_version;
use cw_utils::{must_pay, Expiration};

use crate::{
    execute,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    query,
    state::{Config, FundraiseState, CONFIG, TOTAL_DEPOSITED},
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-fundraise";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    TOTAL_DEPOSITED.save(deps.storage, &Uint128::zero())?;

    // Validation
    let funded = must_pay(&info, &msg.fundraise.denom)?;
    if funded != msg.fundraise.amount {
        return Err(ContractError::StdError(StdError::generic_err(
            "Invalid amount sent",
        )));
    }
    let end = if let Some(start) = msg.start {
        if start.is_expired(&env.block) {
            return Err(ContractError::StdError(StdError::generic_err(
                "Start is already expired",
            )));
        }

        match start {
            Expiration::AtHeight(height) => msg.duration.after(&BlockInfo {
                height,
                time: Timestamp::default(),
                chain_id: String::default(),
            }),
            Expiration::AtTime(time) => msg.duration.after(&BlockInfo {
                height: 0u64,
                time,
                chain_id: String::default(),
            }),
            Expiration::Never {} => {
                return Err(ContractError::StdError(StdError::generic_err(
                    "Start cannot be never",
                )))
            }
        }
    } else {
        msg.duration.after(&env.block)
    };
    if msg.soft_cap.is_zero() {
        return Err(ContractError::StdError(StdError::generic_err(
            "Soft cap must be nonzero",
        )));
    }
    if let Some(hard_cap) = msg.hard_cap {
        if hard_cap <= msg.soft_cap {
            return Err(ContractError::StdError(StdError::generic_err(
                "Hard cap must be greater than the soft cap",
            )));
        }
    }

    CONFIG.save(
        deps.storage,
        &Config {
            state: FundraiseState::Active,
            fundraise: msg.fundraise,
            deposit_denom: msg.deposit_denom,
            soft_cap: msg.soft_cap,
            hard_cap: msg.hard_cap,
            start: msg.start,
            end,
            recipient: info.sender,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => execute::deposit(deps, env, info),
        ExecuteMsg::Withdraw {} => execute::withdraw(deps, env, info),
        ExecuteMsg::Expire {} => execute::expire(deps, env, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query::config(deps)?),
        QueryMsg::TotalDeposited {} => to_json_binary(&query::total_deposited(deps)?),
        QueryMsg::Deposit { addr } => to_json_binary(&query::deposit(deps, addr)?),
        QueryMsg::Reward { addr } => to_json_binary(
            &query::reward(deps, addr).map_err(|e| StdError::generic_err(e.to_string()))?,
        ),
        QueryMsg::DumpState { addr } => to_json_binary(&query::dump_state(deps, addr)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
