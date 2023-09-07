use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, SnapshotItem};

#[cw_serde]
pub struct Ruleset {
    pub rules: Vec<String>,
    pub description: String,
    pub is_enabled: bool,
}

pub struct RulesetIndexes<'a> {
    pub is_enabled: MultiIndex<'a, String, Ruleset, u128>,
}

impl<'a> IndexList<Ruleset> for RulesetIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ruleset>> + '_> {
        let v: Vec<&dyn Index<Ruleset>> = vec![&self.is_enabled];
        Box::new(v.into_iter())
    }
}

pub fn rulesets<'a>() -> IndexedMap<'a, u128, Ruleset, RulesetIndexes<'a>> {
    let indexes = RulesetIndexes {
        is_enabled: MultiIndex::new(
            |_x, d: &Ruleset| d.is_enabled.to_string(),
            "rulesets",
            "rulesets__is_enabled",
        ),
    };
    IndexedMap::new("rulesets", indexes)
}

#[cw_serde]
pub struct CompetitionModule {
    pub key: String,
    pub addr: Addr,
    pub is_enabled: bool,
}

pub struct CompetitionModuleIndexes<'a> {
    pub is_enabled: MultiIndex<'a, String, CompetitionModule, Addr>,
}

impl<'a> IndexList<CompetitionModule> for CompetitionModuleIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CompetitionModule>> + '_> {
        let v: Vec<&dyn Index<CompetitionModule>> = vec![&self.is_enabled];
        Box::new(v.into_iter())
    }
}

pub fn competition_modules<'a>(
) -> IndexedMap<'a, Addr, CompetitionModule, CompetitionModuleIndexes<'a>> {
    let indexes = CompetitionModuleIndexes {
        is_enabled: MultiIndex::new(
            |_x, d: &CompetitionModule| d.is_enabled.to_string(),
            "competition_modules",
            "competition_modules__is_enabled",
        ),
    };
    IndexedMap::new("competition_modules", indexes)
}

pub const COMPETITION_MODULES_COUNT: Item<Uint128> = Item::new("competition-modules-count");
pub const TAX: SnapshotItem<Decimal> = SnapshotItem::new(
    "tax",
    "tax__check",
    "tax__change",
    cw_storage_plus::Strategy::EveryBlock,
);
pub const RULESET_COUNT: Item<Uint128> = Item::new("ruleset_count");
pub const KEYS: Map<String, Addr> = Map::new("keys");
