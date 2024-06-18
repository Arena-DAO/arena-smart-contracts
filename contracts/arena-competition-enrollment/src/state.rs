use std::fmt;

use arena_tournament_module::state::EliminationType;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128, Uint64};
use cw_address_like::AddressLike;
use cw_storage_plus::Map;
use cw_utils::Expiration;

#[cw_serde]
pub struct EnrollmentEntry<T: AddressLike> {
    pub id: Uint128,
    pub min_members: Option<Uint128>,
    pub max_members: Uint128,
    pub entry_fee: Option<Coin>,
    pub expiration: Expiration,
    pub has_triggered_expiration: bool,
    pub category_id: Option<Uint128>,
    pub competition_info: CompetitionInfo<T>,
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
pub enum CompetitionInfo<T: AddressLike> {
    Pending {
        host: T,
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

pub const CATEGORY_COMPETITION_COUNT: Map<u128, Uint128> = Map::new("category_competition_count");
