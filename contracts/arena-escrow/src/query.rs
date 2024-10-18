use arena_interface::escrow::DumpStateResponse;
use cosmwasm_std::{Deps, StdResult};
use cw_balance::{BalanceVerified, MemberBalanceChecked};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::state::{BALANCE, DUE, INITIAL_DUE, IS_LOCKED, TOTAL_BALANCE};

pub fn balance(deps: Deps, addr: String) -> StdResult<Option<BalanceVerified>> {
    let addr = deps.api.addr_validate(&addr)?;

    BALANCE.may_load(deps.storage, &addr)
}

pub fn due(deps: Deps, addr: String) -> StdResult<Option<BalanceVerified>> {
    let addr = deps.api.addr_validate(&addr)?;
    DUE.may_load(deps.storage, &addr)
}

pub fn total_balance(deps: Deps) -> StdResult<Option<BalanceVerified>> {
    TOTAL_BALANCE.may_load(deps.storage)
}

pub fn is_locked(deps: Deps) -> bool {
    IS_LOCKED.load(deps.storage).unwrap_or_default()
}

pub fn is_funded(deps: Deps, addr: String) -> StdResult<bool> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(crate::state::is_funded(deps, &addr))
}

pub fn balances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceChecked>> {
    let binding = maybe_addr(deps.api, start_after)?;
    let start = binding.as_ref().map(Bound::exclusive);

    cw_paginate::paginate_map(&BALANCE, deps.storage, start, limit, |k, v| {
        Ok(MemberBalanceChecked {
            addr: k,
            balance: v,
        })
    })
}

pub fn dues(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceChecked>> {
    let binding = maybe_addr(deps.api, start_after)?;
    let start = binding.as_ref().map(Bound::exclusive);
    cw_paginate::paginate_map(&DUE, deps.storage, start, limit, |k, v| {
        Ok(MemberBalanceChecked {
            addr: k,
            balance: v,
        })
    })
}

pub fn initial_dues(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceChecked>> {
    let binding = maybe_addr(deps.api, start_after)?;
    let start = binding.as_ref().map(Bound::exclusive);
    cw_paginate::paginate_map(&INITIAL_DUE, deps.storage, start, limit, |k, v| {
        Ok(MemberBalanceChecked {
            addr: k,
            balance: v,
        })
    })
}

pub fn dump_state(deps: Deps, addr: Option<String>) -> StdResult<DumpStateResponse> {
    let maybe_addr = maybe_addr(deps.api, addr)?;
    let balance = maybe_addr
        .as_ref()
        .map(|x| balance(deps, x.to_string()))
        .transpose()?
        .flatten();
    let due = maybe_addr
        .map(|x| due(deps, x.to_string()))
        .transpose()?
        .flatten();

    Ok(DumpStateResponse {
        due,
        is_locked: is_locked(deps),
        total_balance: total_balance(deps)?,
        balance,
    })
}
