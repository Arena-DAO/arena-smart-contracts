use cosmwasm_std::{Addr, Deps, StdResult};
use cw_balance::{BalanceVerified, MemberShareVerified};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::state::{BALANCE, DUE, IS_FUNDED, IS_LOCKED, PRESET_DISTRIBUTION, TOTAL_BALANCE};

pub fn balance(deps: Deps, addr: String) -> StdResult<BalanceVerified> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(BALANCE.may_load(deps.storage, &addr)?.unwrap_or_default())
}

pub fn due(deps: Deps, addr: String) -> StdResult<BalanceVerified> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(DUE.may_load(deps.storage, &addr)?.unwrap_or_default())
}

pub fn total_balance(deps: Deps) -> BalanceVerified {
    TOTAL_BALANCE.load(deps.storage).unwrap_or_default()
}

pub fn is_locked(deps: Deps) -> bool {
    IS_LOCKED.load(deps.storage).unwrap_or_default()
}

pub fn distribution(deps: Deps, addr: String) -> StdResult<Option<Vec<MemberShareVerified>>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(PRESET_DISTRIBUTION.may_load(deps.storage, &addr)?)
}

pub fn is_funded(deps: Deps, addr: String) -> StdResult<bool> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(IS_FUNDED.load(deps.storage, &addr).unwrap_or_default())
}

pub fn is_fully_funded(deps: Deps) -> StdResult<bool> {
    Ok(crate::state::is_fully_funded(deps)?)
}

pub fn balances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<(Addr, BalanceVerified)>> {
    let binding = maybe_addr(deps.api, start_after)?;
    let start = binding.as_ref().map(Bound::exclusive);
    cw_paginate::paginate_map(&BALANCE, deps.storage, start, limit, |k, v| Ok((k, v)))
}

pub fn dues(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<(Addr, BalanceVerified)>> {
    let binding = maybe_addr(deps.api, start_after)?;
    let start = binding.as_ref().map(Bound::exclusive);
    cw_paginate::paginate_map(&DUE, deps.storage, start, limit, |k, v| Ok((k, v)))
}
