use cosmwasm_std::{Addr, Decimal, Deps};
use cw_balance::{BalanceVerified, Distribution};
use cw_storage_plus::{Item, Map};

pub const TOTAL_BALANCE: Item<BalanceVerified> = Item::new("total");
pub const BALANCE: Map<&Addr, BalanceVerified> = Map::new("balance");
pub const INITIAL_DUE: Map<&Addr, BalanceVerified> = Map::new("initial_due");
pub const DUE: Map<&Addr, BalanceVerified> = Map::new("due");
pub const IS_LOCKED: Item<bool> = Item::new("is_locked");
pub const HAS_DISTRIBUTED: Item<bool> = Item::new("has_distributed");
pub const PRESET_DISTRIBUTION: Map<&Addr, Distribution<Addr>> = Map::new("distribution");
pub const TAX_AT_WITHDRAWAL: Item<Decimal> = Item::new("tax_at_withdrawal");

pub fn is_fully_funded(deps: Deps) -> bool {
    DUE.is_empty(deps.storage)
}

pub fn is_funded(deps: Deps, addr: &Addr) -> bool {
    !DUE.has(deps.storage, addr)
}
