use cosmwasm_std::{Deps, Env, StdError, StdResult, Uint128};

use crate::{
    msg::DumpStateResponse,
    state::{Config, CONFIG, TOTAL_DEPOSITED, USER_DEPOSIT},
    ContractError,
};

pub fn config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn total_deposited(deps: Deps) -> StdResult<Uint128> {
    TOTAL_DEPOSITED.load(deps.storage)
}

pub fn deposit(deps: Deps, addr: String) -> StdResult<Option<Uint128>> {
    let addr = deps.api.addr_validate(&addr)?;

    USER_DEPOSIT.may_load(deps.storage, &addr)
}

pub fn reward(deps: Deps, addr: String) -> Result<Option<Uint128>, ContractError> {
    match deposit(deps, addr)? {
        Some(deposit) => {
            let config = config(deps)?;
            let total_deposited = total_deposited(deps)?;

            let reward = config
                .fundraise
                .amount
                .checked_mul_floor((deposit, total_deposited))?;

            Ok(Some(reward))
        }
        None => Ok(None),
    }
}

pub fn dump_state(deps: Deps, env: Env, addr: Option<String>) -> StdResult<DumpStateResponse> {
    let config = config(deps)?;
    let total_deposited = total_deposited(deps)?;
    let has_expired = config.end.is_expired(&env.block);
    let has_started = config.start.map(|x| x.is_expired(&env.block));

    match addr {
        Some(addr) => Ok(DumpStateResponse {
            config,
            total_deposited,
            deposit: deposit(deps, addr.clone())?,
            reward: reward(deps, addr).map_err(|e| StdError::generic_err(e.to_string()))?,
            has_expired,
            has_started,
        }),
        None => Ok(DumpStateResponse {
            config,
            total_deposited,
            deposit: None,
            reward: None,
            has_expired,
            has_started,
        }),
    }
}
