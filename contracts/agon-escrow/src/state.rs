use cosmwasm_std::{Addr, Deps};
use cw_balance::{Balance, Distribution};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

use crate::ContractError;

pub const ADMIN: Admin = Admin::new("admin");
pub const TOTAL_BALANCE: Item<Balance> = Item::new("total");
pub const BALANCE: Map<&Addr, Balance> = Map::new("balance");
pub const DUE: Map<&Addr, Balance> = Map::new("due");
pub const STAKE: Map<&Addr, Balance> = Map::new("stake");
pub const TOTAL_STAKE: Item<Balance> = Item::new("total_stake");
pub const IS_LOCKED: Item<bool> = Item::new("is_locked");
pub const DISTRIBUTION: Map<&Addr, Distribution> = Map::new("distribution");

pub fn get_distributable_balance(deps: Deps) -> Result<Balance, ContractError> {
    let total_balance = TOTAL_BALANCE.load(deps.storage)?;
    let total_stake = TOTAL_STAKE.load(deps.storage)?;

    Ok(total_balance.checked_sub(&total_stake)?)
}
