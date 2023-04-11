use cosmwasm_std::{Deps, StdResult};
use cw_balance::{Balance, Distribution, DistributionRaw};

use crate::{
    msg::DumpStateResponse,
    state::{ADMIN, BALANCE, DISTRIBUTION, DUE, IS_LOCKED, STAKE, TOTAL_BALANCE},
};

pub fn balance(deps: Deps, addr: String) -> StdResult<Balance> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(BALANCE.may_load(deps.storage, &addr)?.unwrap_or_default())
}

pub fn due(deps: Deps, addr: String) -> StdResult<Balance> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(DUE.may_load(deps.storage, &addr)?.unwrap_or_default())
}

pub fn total_balance(deps: Deps) -> Balance {
    TOTAL_BALANCE.load(deps.storage).unwrap_or_default()
}

pub fn stake(deps: Deps, addr: String) -> StdResult<Balance> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(STAKE.may_load(deps.storage, &addr)?.unwrap_or_default())
}

pub fn is_locked(deps: Deps) -> bool {
    IS_LOCKED.load(deps.storage).unwrap_or_default()
}

pub fn dump_state(deps: Deps) -> StdResult<DumpStateResponse> {
    Ok(DumpStateResponse {
        admin: ADMIN.get(deps)?.unwrap(),
        is_locked: is_locked(deps),
        total_balance: total_balance(deps),
    })
}

pub fn distribution(deps: Deps, addr: String) -> StdResult<Option<Distribution>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(DISTRIBUTION.may_load(deps.storage, &addr)?)
}
