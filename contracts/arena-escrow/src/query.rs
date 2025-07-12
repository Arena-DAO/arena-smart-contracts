use arena_interface::escrow::DumpStateResponse;
use cosmwasm_std::{Deps, StdResult};
use cw_balance::{BalanceVerified, MemberBalanceChecked};
use cw_utils::maybe_addr;

use crate::{
    balance_manager::{BalanceManager, TotalBalanceManager},
    state::{
        paginate_balances_for, BALANCE_CW20, BALANCE_CW721, BALANCE_NATIVE, DUE_CW20, DUE_CW721,
        DUE_NATIVE, INITIAL_DUE_CW20, INITIAL_DUE_CW721, INITIAL_DUE_NATIVE, IS_LOCKED,
    },
};

pub fn balance(deps: Deps, addr: String) -> StdResult<BalanceVerified> {
    let addr = deps.api.addr_validate(&addr)?;

    BalanceManager::new(&BALANCE_NATIVE, &BALANCE_CW20, &BALANCE_CW721).load_balance(deps, &addr)
}

pub fn due(deps: Deps, addr: String) -> StdResult<BalanceVerified> {
    let addr = deps.api.addr_validate(&addr)?;

    BalanceManager::new(&DUE_NATIVE, &DUE_CW20, &DUE_CW721).load_balance(deps, &addr)
}

pub fn total_balance(deps: Deps) -> StdResult<BalanceVerified> {
    TotalBalanceManager::load(deps)
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
    paginate_balances_for(
        deps,
        &BALANCE_NATIVE,
        &BALANCE_CW20,
        &BALANCE_CW721,
        start_after,
        limit,
    )
}

pub fn dues(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceChecked>> {
    paginate_balances_for(deps, &DUE_NATIVE, &DUE_CW20, &DUE_CW721, start_after, limit)
}

pub fn initial_dues(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceChecked>> {
    paginate_balances_for(
        deps,
        &INITIAL_DUE_NATIVE,
        &INITIAL_DUE_CW20,
        &INITIAL_DUE_CW721,
        start_after,
        limit,
    )
}

pub fn dump_state(deps: Deps, addr: Option<String>) -> StdResult<DumpStateResponse> {
    let maybe_addr = maybe_addr(deps.api, addr)?;
    let balance = maybe_addr
        .as_ref()
        .map(|x| balance(deps, x.to_string()))
        .transpose()?
        .unwrap_or_default();
    let due = maybe_addr
        .map(|x| due(deps, x.to_string()))
        .transpose()?
        .unwrap_or_default();

    Ok(DumpStateResponse {
        due,
        is_locked: is_locked(deps),
        total_balance: total_balance(deps)?,
        balance,
    })
}
