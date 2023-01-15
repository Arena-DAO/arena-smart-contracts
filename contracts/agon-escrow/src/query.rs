use cosmwasm_std::{Deps, StdResult};
use cw_tokens::GenericTokenBalance;

use crate::state::{BALANCE, DUE, TOTAL_BALANCE};

pub fn balance(deps: Deps, addr: String) -> StdResult<Vec<GenericTokenBalance>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(BALANCE.load(deps.storage, addr)?)
}

pub fn due(deps: Deps, addr: String) -> StdResult<Vec<GenericTokenBalance>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(DUE.load(deps.storage, addr)?)
}

pub fn total(deps: Deps) -> StdResult<Vec<GenericTokenBalance>> {
    Ok(TOTAL_BALANCE.load(deps.storage)?)
}
