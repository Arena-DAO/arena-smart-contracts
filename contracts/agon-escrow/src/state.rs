use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use cw_tokens::GenericTokenBalance;

use crate::models::EscrowState;

pub const ADMIN: Admin = Admin::new("admin");
pub const TOTAL_BALANCE: Item<Vec<GenericTokenBalance>> = Item::new("total");
pub const BALANCE: Map<Addr, Vec<GenericTokenBalance>> = Map::new("balance");
pub const DUE: Map<Addr, Vec<GenericTokenBalance>> = Map::new("due");
pub const STAKE: Map<Addr, Vec<GenericTokenBalance>> = Map::new("stake");
pub const STATE: Item<EscrowState> = Item::new("state");
pub const KEY: Item<String> = Item::new("key");
