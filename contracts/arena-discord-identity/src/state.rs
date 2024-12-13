use cosmwasm_std::{Addr, Coin, Uint64};
use cw_storage_plus::{Item, Map};

/// Maps an address to discord user id
pub const DISCORD_IDENTITY: Map<&Addr, Uint64> = Map::new("discord_identity");
pub const REVERSE_IDENTITY_MAP: Map<u64, Addr> = Map::new("reverse_identity_map");
pub const FAUCET_AMOUNT: Item<Coin> = Item::new("faucet_amount");
