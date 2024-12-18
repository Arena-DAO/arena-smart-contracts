use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

use crate::msg::{DiscordConnection, DiscordProfile};

pub struct DiscordProfileIndexes<'a> {
    pub discord_id: MultiIndex<'a, u64, DiscordProfile, &'a Addr>,
}

impl IndexList<DiscordProfile> for DiscordProfileIndexes<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<DiscordProfile>> + '_> {
        let v: Vec<&dyn Index<DiscordProfile>> = vec![&self.discord_id];
        Box::new(v.into_iter())
    }
}

pub fn discord_identity<'a>() -> IndexedMap<'a, &'a Addr, DiscordProfile, DiscordProfileIndexes<'a>>
{
    let indexes = DiscordProfileIndexes {
        discord_id: MultiIndex::new(
            |_, d: &DiscordProfile| d.user_id.u64(),
            "discord_identity",
            "discord_identity__discord_id",
        ),
    };
    IndexedMap::new("discord_identity", indexes)
}

pub const DISCORD_CONNECTIONS: Map<u64, Vec<DiscordConnection>> = Map::new("discord_connections");
pub const FAUCET_AMOUNT: Item<Coin> = Item::new("faucet_amount");
