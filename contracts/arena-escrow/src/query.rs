use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult};
use cw_balance::{BalanceVerified, MemberBalanceVerified, MemberShare};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::state::{BALANCE, DUE, IS_FUNDED, IS_LOCKED, PRESET_DISTRIBUTION, TOTAL_BALANCE};

#[cw_serde]
pub struct DumpStateResponse {
    pub dues: Vec<MemberBalanceVerified>,
    pub is_locked: bool,
    pub total_balance: BalanceVerified,
    pub balance: BalanceVerified,
}

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

pub fn distribution(deps: Deps, addr: String) -> StdResult<Option<Vec<MemberShare<Addr>>>> {
    let addr = deps.api.addr_validate(&addr)?;
    PRESET_DISTRIBUTION.may_load(deps.storage, &addr)
}

pub fn is_funded(deps: Deps, addr: String) -> StdResult<bool> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(IS_FUNDED.load(deps.storage, &addr).unwrap_or_default())
}

pub fn is_fully_funded(deps: Deps) -> StdResult<bool> {
    crate::state::is_fully_funded(deps)
}

pub fn balances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceVerified>> {
    let binding = maybe_addr(deps.api, start_after)?;
    let start = binding.as_ref().map(Bound::exclusive);
    cw_paginate::paginate_map(&BALANCE, deps.storage, start, limit, |k, v| {
        Ok(MemberBalanceVerified {
            addr: k,
            balance: v,
        })
    })
}

pub fn dues(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceVerified>> {
    let binding = maybe_addr(deps.api, start_after)?;
    let start = binding.as_ref().map(Bound::exclusive);
    cw_paginate::paginate_map(&DUE, deps.storage, start, limit, |k, v| {
        Ok(MemberBalanceVerified {
            addr: k,
            balance: v,
        })
    })
}

pub fn dump_state(deps: Deps, addr: Option<String>) -> StdResult<DumpStateResponse> {
    let maybe_addr = maybe_addr(deps.api, addr)?;
    let balance = maybe_addr
        .map(|x| balance(deps, x.to_string()))
        .transpose()?
        .unwrap_or_default();

    Ok(DumpStateResponse {
        dues: dues(deps, None, None)?,
        is_locked: is_locked(deps),
        total_balance: total_balance(deps),
        balance,
    })
}
