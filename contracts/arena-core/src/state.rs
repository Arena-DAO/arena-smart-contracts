use arena_interface::{
    core::{CompetitionCategory, Ruleset},
    fees::TaxConfiguration,
    ratings::Rating,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, SnapshotItem, SnapshotMap};
use cw_utils::Duration;

pub const ARENA_TAX_CONFIG: Item<TaxConfiguration> = Item::new("arena_tax_config");
pub const COMPETITION_CATEGORIES_COUNT: Item<Uint128> = Item::new("competition_categories_count");
pub const COMPETITION_MODULES_COUNT: Item<Uint128> = Item::new("competition_modules_count");
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
pub const RATING_PERIOD: Item<Duration> = Item::new("rating_period");

// Competition Modules
#[cw_serde]
pub struct CompetitionModule {
    pub key: String,
    pub addr: Addr,
    pub is_enabled: bool,
}

pub struct CompetitionModuleIndexes<'a> {
    pub is_enabled: MultiIndex<'a, String, CompetitionModule, &'a Addr>,
}

impl<'a> IndexList<CompetitionModule> for CompetitionModuleIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CompetitionModule>> + '_> {
        let v: Vec<&dyn Index<CompetitionModule>> = vec![&self.is_enabled];
        Box::new(v.into_iter())
    }
}

pub fn competition_modules<'a>(
) -> IndexedMap<'a, &'a Addr, CompetitionModule, CompetitionModuleIndexes<'a>> {
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

// Ratings

pub struct RatingIndexes<'a> {
    pub rating: MultiIndex<'a, u128, Rating, (u128, &'a Addr)>, // We want to be able to sort by rating value
}

impl<'a> IndexList<Rating> for RatingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Rating>> + '_> {
        let v: Vec<&dyn Index<Rating>> = vec![&self.rating];
        Box::new(v.into_iter())
    }
}

// Ratings are stored by category id and address
pub fn ratings<'a>() -> IndexedMap<'a, (u128, &'a Addr), Rating, RatingIndexes<'a>> {
    let indexes = RatingIndexes {
        rating: MultiIndex::new(
            |_x, d: &Rating| d.value.atomics().u128(),
            "ratings",
            "ratings__rating",
        ),
    };
    IndexedMap::new("ratings", indexes)
}
