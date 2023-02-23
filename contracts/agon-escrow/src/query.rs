use cosmwasm_std::{Deps, StdResult};
use cw_tokens::GenericTokenBalance;

use crate::{
    models::EscrowState,
    state::{BALANCE, DUE, STAKE, STATE, TOTAL_BALANCE},
};

pub fn balance(deps: Deps, addr: String) -> StdResult<Vec<GenericTokenBalance>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(BALANCE.may_load(deps.storage, addr)?.unwrap_or_default())
}

pub fn due(deps: Deps, addr: String) -> StdResult<Vec<GenericTokenBalance>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(DUE.may_load(deps.storage, addr)?.unwrap_or_default())
}

pub fn total(deps: Deps) -> StdResult<Vec<GenericTokenBalance>> {
    Ok(TOTAL_BALANCE.may_load(deps.storage)?.unwrap_or_default())
}

pub fn stake(deps: Deps, addr: String) -> StdResult<Vec<GenericTokenBalance>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(STAKE.may_load(deps.storage, addr)?.unwrap_or_default())
}

pub fn state(deps: Deps) -> StdResult<EscrowState> {
    Ok(STATE
        .may_load(deps.storage)?
        .unwrap_or(EscrowState::Unlocked {}))
}
