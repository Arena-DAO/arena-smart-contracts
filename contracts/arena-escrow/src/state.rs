use std::collections::BTreeSet;

use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};
use cw_balance::MemberBalanceChecked;
use cw_storage_plus::{Item, Map};
use cw_utils::maybe_addr;

use crate::balance_manager::BalanceManager;

// Enrollment and lock state
pub const ENROLLMENT_CONTRACT: Item<Addr> = Item::new("enrollment_contract");
pub const IS_LOCKED: Item<bool> = Item::new("is_locked");
pub const HAS_DISTRIBUTED: Item<bool> = Item::new("has_distributed");

// Native token balances: (user, denom) -> amount
pub const BALANCE_NATIVE: Map<(&Addr, &str), Uint128> = Map::new("balance_native");
pub const DUE_NATIVE: Map<(&Addr, &str), Uint128> = Map::new("due_native");
pub const INITIAL_DUE_NATIVE: Map<(&Addr, &str), Uint128> = Map::new("initial_due_native");
pub const TOTAL_BALANCE_NATIVE: Map<&str, Uint128> = Map::new("total_balance_native");

// CW20 token balances: (user, token_addr) -> amount
pub const TOTAL_BALANCE_CW20: Map<&Addr, Uint128> = Map::new("total_balance_cw20");
pub const BALANCE_CW20: Map<(&Addr, &Addr), Uint128> = Map::new("balance_cw20");
pub const DUE_CW20: Map<(&Addr, &Addr), Uint128> = Map::new("due_cw20");
pub const INITIAL_DUE_CW20: Map<(&Addr, &Addr), Uint128> = Map::new("initial_due_cw20");

// CW721 token balances: (user, contract, token_id) -> ()
pub const TOTAL_BALANCE_CW721: Map<(&Addr, &str), ()> = Map::new("total_balance_cw721");
pub const BALANCE_CW721: Map<(&Addr, &Addr, &str), ()> = Map::new("balance_cw721");
pub const DUE_CW721: Map<(&Addr, &Addr, &str), ()> = Map::new("due_cw721");
pub const INITIAL_DUE_CW721: Map<(&Addr, &Addr, &str), ()> = Map::new("initial_due_cw721");

// Derived helper: fully funded if no due entries
pub fn is_fully_funded(deps: Deps) -> bool {
    DUE_NATIVE.is_empty(deps.storage)
        && DUE_CW20.is_empty(deps.storage)
        && DUE_CW721.is_empty(deps.storage)
}

// Check funding for a user (partial check — all 3 types can be added if needed)
pub fn is_funded(deps: Deps, addr: &Addr) -> bool {
    DUE_NATIVE.prefix(addr).is_empty(deps.storage)
        && DUE_CW20.prefix(addr).is_empty(deps.storage)
        && DUE_CW721.sub_prefix(addr).is_empty(deps.storage)
}

pub fn paginate_balances_for(
    deps: Deps,
    native_map: &Map<(&Addr, &str), Uint128>,
    cw20_map: &Map<(&Addr, &Addr), Uint128>,
    cw721_map: &Map<(&Addr, &Addr, &str), ()>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberBalanceChecked>> {
    let limit = limit
        .unwrap_or(cw_paginate::DEFAULT_LIMIT)
        .min(cw_paginate::MAX_LIMIT) as usize;
    let start = maybe_addr(deps.api, start_after)?;

    let iter = native_map
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(a, _)| a))
        .chain(
            cw20_map
                .keys(deps.storage, None, None, Order::Ascending)
                .map(|r| r.map(|(a, _)| a)),
        )
        .chain(
            cw721_map
                .keys(deps.storage, None, None, Order::Ascending)
                .map(|r| r.map(|(a, _, _)| a)),
        );

    let mut seen = BTreeSet::new();
    for addr in iter {
        let addr = addr?;
        if start.as_ref().map_or(true, |s| addr > *s) {
            seen.insert(addr);
            if seen.len() == limit {
                break;
            }
        }
    }

    let balance_manager = BalanceManager::new(native_map, cw20_map, cw721_map);
    seen.into_iter()
        .map(|addr| {
            let balance = balance_manager.load_balance(deps, &addr)?;
            Ok(MemberBalanceChecked { addr, balance })
        })
        .collect()
}
