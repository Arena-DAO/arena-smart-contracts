use cosmwasm_std::{Addr, Deps, StdResult};
use cw_balance::{BalanceVerified, MemberShareVerified};
use cw_storage_plus::{Item, Map};

pub const TOTAL_BALANCE: Item<BalanceVerified> = Item::new("total");
pub const BALANCE: Map<&Addr, BalanceVerified> = Map::new("balance");
pub const DUE: Map<&Addr, BalanceVerified> = Map::new("due");
pub const IS_LOCKED: Item<bool> = Item::new("is_locked");
pub const PRESET_DISTRIBUTION: Map<&Addr, Vec<MemberShareVerified>> = Map::new("distribution");
pub const IS_FUNDED: Map<&Addr, bool> = Map::new("is_funded");

pub fn is_fully_funded(deps: Deps) -> StdResult<bool> {
    // Load all funded bits
    let is_funded = IS_FUNDED
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(Addr, bool)>>>()?;

    // Iterate through each entry
    for (_addr, value) in is_funded {
        if !value {
            return Ok(false);
        }
    }

    Ok(true)
}
