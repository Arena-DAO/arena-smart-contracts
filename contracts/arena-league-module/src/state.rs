use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Uint128, Uint64};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use cw_utils::Expiration;

pub struct RoundIndexes<'a> {
    pub expiration: MultiIndex<'a, String, Round, (u128, u64)>,
}

impl<'a> IndexList<Round> for RoundIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Round>> + '_> {
        let v: Vec<&dyn Index<Round>> = vec![&self.expiration];
        Box::new(v.into_iter())
    }
}

// Map is stored by (competition_id, round_number)
pub fn rounds<'a>() -> IndexedMap<'a, (u128, u64), Round, RoundIndexes<'a>> {
    let indexes = RoundIndexes {
        expiration: MultiIndex::new(
            |_x, d: &Round| d.expiration.to_string(),
            "rounds",
            "rounds__expiration",
        ),
    };

    IndexedMap::new("rounds", indexes)
}

#[cw_serde]
pub struct MatchResult {
    // Some(BOOL) is a winner where true is team1 and false is team2, and None is a draw
    pub result: Option<bool>,
    pub block: BlockInfo,
}

#[cw_serde]
pub struct Match {
    // This is a reference to the wager_module's generated wager id
    pub wager_id: Uint128,
    pub match_number: Uint128,
    pub team_1: Addr,
    pub team_2: Addr,
    pub result: Option<MatchResult>,
}

#[cw_serde]
pub struct Round {
    pub round_number: Uint64,
    pub matches: Vec<Match>,
    pub expiration: Expiration,
}

pub const WAGERS_KEY: Item<String> = Item::new("wagers_key");
