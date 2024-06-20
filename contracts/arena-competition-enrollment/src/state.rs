use std::fmt;

use arena_tournament_module::state::EliminationType;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128, Uint64};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use cw_utils::Expiration;

#[cw_serde]
pub struct EnrollmentEntry {
    pub id: Uint128,
    pub min_members: Option<Uint128>,
    pub max_members: Uint128,
    pub entry_fee: Option<Coin>,
    pub expiration: Expiration,
    pub has_triggered_expiration: bool,
    pub competition_info: CompetitionInfo,
    pub host: Addr,
}

#[cw_serde]
pub enum CompetitionType {
    Wager {},
    League {
        match_win_points: Uint64,
        match_draw_points: Uint64,
        match_lose_points: Uint64,
        distribution: Vec<Decimal>,
    },
    Tournament {
        elimination_type: EliminationType,
        distribution: Vec<Decimal>,
    },
}

impl fmt::Display for CompetitionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompetitionType::Wager {} => write!(f, "wager"),
            CompetitionType::League { .. } => write!(f, "league"),
            CompetitionType::Tournament { .. } => write!(f, "tournament"),
        }
    }
}

#[cw_serde]
pub enum CompetitionInfo {
    Pending {
        name: String,
        description: String,
        expiration: Expiration,
        rules: Vec<String>,
        rulesets: Vec<Uint128>,
        banner: Option<String>,
        competition_type: CompetitionType,
    },
    Existing(Addr),
}

pub struct EnrollmentEntryIndexes<'a> {
    pub expiration: MultiIndex<'a, String, EnrollmentEntry, (u128, u128)>,
    pub host: MultiIndex<'a, String, EnrollmentEntry, (u128, u128)>,
}

impl<'a> IndexList<EnrollmentEntry> for EnrollmentEntryIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<EnrollmentEntry>> + '_> {
        let v: Vec<&dyn Index<EnrollmentEntry>> = vec![&self.expiration, &self.host];
        Box::new(v.into_iter())
    }
}

/// Enrollment entries are stored by category id and entry id
pub fn enrollment_entries<'a>(
) -> IndexedMap<'a, (u128, u128), EnrollmentEntry, EnrollmentEntryIndexes<'a>> {
    let indexes = EnrollmentEntryIndexes {
        expiration: MultiIndex::new(
            |_x, d: &EnrollmentEntry| d.expiration.to_string(),
            "enrollment_entries",
            "enrollment_entries__expiration",
        ),
        host: MultiIndex::new(
            |_x, d: &EnrollmentEntry| d.host.to_string(),
            "enrollment_entries",
            "enrollment_entries__host",
        ),
    };
    IndexedMap::new("enrollment_entries", indexes)
}

/// None category will be 0
pub const CATEGORY_COMPETITION_COUNT: Map<u128, Uint128> = Map::new("category_competition_count");
