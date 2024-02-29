use arena_core_interface::msg::{CompetitionCategory, Ruleset};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, SnapshotItem, SnapshotMap};

pub const COMPETITION_CATEGORIES_COUNT: Item<Uint128> = Item::new("competition-categories-count");
pub const COMPETITION_MODULES_COUNT: Item<Uint128> = Item::new("competition-modules-count");
pub const TAX: SnapshotItem<Decimal> = SnapshotItem::new(
    "tax",
    "tax__check",
    "tax__change",
    cw_storage_plus::Strategy::EveryBlock,
);
pub const RULESETS_COUNT: Item<Uint128> = Item::new("ruleset_count");
pub const KEYS: SnapshotMap<String, Addr> = SnapshotMap::new(
    "keys",
    "keys__check",
    "keys__change",
    cw_storage_plus::Strategy::EveryBlock,
);

// Competition Modules

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

// Competition Categories

pub struct CompetitionCategoryIndexes<'a> {
    pub is_enabled: MultiIndex<'a, String, CompetitionCategory, u128>,
}

impl<'a> IndexList<CompetitionCategory> for CompetitionCategoryIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CompetitionCategory>> + '_> {
        let v: Vec<&dyn Index<CompetitionCategory>> = vec![&self.is_enabled];
        Box::new(v.into_iter())
    }
}

pub fn competition_categories<'a>(
) -> IndexedMap<'a, u128, CompetitionCategory, CompetitionCategoryIndexes<'a>> {
    let indexes = CompetitionCategoryIndexes {
        is_enabled: MultiIndex::new(
            |_x, d: &CompetitionCategory| d.is_enabled.to_string(),
            "competition_categories",
            "competition_categories__is_enabled",
        ),
    };
    IndexedMap::new("competition_categories", indexes)
}

// Rulesets

pub fn get_rulesets_category_and_is_enabled_idx(
    category_id: Option<Uint128>,
    is_enabled: bool,
) -> String {
    format!("{:?}_{}", category_id, is_enabled)
}

pub struct RulesetIndexes<'a> {
    pub category_and_is_enabled: MultiIndex<'a, String, Ruleset, u128>,
}

impl<'a> IndexList<Ruleset> for RulesetIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ruleset>> + '_> {
        let v: Vec<&dyn Index<Ruleset>> = vec![&self.category_and_is_enabled];
        Box::new(v.into_iter())
    }
}

pub fn rulesets<'a>() -> IndexedMap<'a, u128, Ruleset, RulesetIndexes<'a>> {
    let indexes = RulesetIndexes {
        category_and_is_enabled: MultiIndex::new(
            |_x, d: &Ruleset| get_rulesets_category_and_is_enabled_idx(d.category_id, d.is_enabled),
            "rulesets",
            "rulesets__category_and_is_enabled",
        ),
    };
    IndexedMap::new("rulesets", indexes)
}
