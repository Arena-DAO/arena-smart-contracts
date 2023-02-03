use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, SnapshotItem};

use crate::models::{CompetitionModule, Ruleset, Wager};

pub struct RulesetIndexes<'a> {
    pub description: MultiIndex<'a, String, Ruleset, u128>,
}

impl<'a> IndexList<Ruleset> for RulesetIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ruleset>> + '_> {
        let v: Vec<&dyn Index<Ruleset>> = vec![&self.description];
        Box::new(v.into_iter())
    }
}

pub fn rulesets<'a>() -> IndexedMap<'a, u128, Ruleset, RulesetIndexes<'a>> {
    let indexes = RulesetIndexes {
        description: MultiIndex::new(
            |_x, d: &Ruleset| d.description.clone(),
            "ruleset",
            "ruleset__description",
        ),
    };
    IndexedMap::new("rulesets", indexes)
}

pub const DAO: Item<Addr> = Item::new("dao");
//maps a name key to an active competition module
pub const COMPETITION_MODULES: Map<Addr, CompetitionModule> = Map::new("competition-modules");
pub const COMPETITION_MODULES_COUNT: Item<Uint128> = Item::new("competition-modules-count");
pub const WAGERS: Map<u128, Wager> = Map::new("wagers");
pub const WAGER_COUNT: Item<Uint128> = Item::new("wager-count");
pub const TEMP_WAGER: Item<u128> = Item::new("temp_wager");
pub const TAX: SnapshotItem<Decimal> = SnapshotItem::new(
    "tax",
    "tax__check",
    "tax__change",
    cw_storage_plus::Strategy::EveryBlock,
);
pub const PROPOSAL_MODULE: Item<Addr> = Item::new("proposal-module");
