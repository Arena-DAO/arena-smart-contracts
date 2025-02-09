use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128, Uint64};

use crate::fees::FeeInformation;

use super::state::CompetitionResponse;

// WAGERS

#[cw_serde]
pub struct WagerExt {}

pub type WagerResponse = CompetitionResponse<WagerExt>;

// LEAGUES

#[cw_serde]
pub struct LeagueExt {
    pub match_win_points: Uint64,
    pub match_draw_points: Uint64,
    pub match_lose_points: Uint64,
    pub rounds: Uint64,
    pub matches: Uint128,
    pub teams: Uint64,
    pub processed_matches: Uint128,
    pub distribution: Vec<Decimal>,
}

pub type LeagueResponse = CompetitionResponse<LeagueExt>;

// TOURNAMENTS

#[cw_serde]
#[derive(Copy)]
pub enum EliminationType {
    SingleElimination { play_third_place_match: bool },
    DoubleElimination,
}

#[cw_serde]
pub struct TournamentExt {
    pub elimination_type: EliminationType, // Enum for single or double elimination
    pub distribution: Vec<Decimal>,
    pub total_matches: Uint128,
    pub processed_matches: Uint128,
}

pub type TournamentResponse = CompetitionResponse<TournamentExt>;

// Enrollments

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
    /// This value should match up the module key
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompetitionType::Wager {} => write!(f, "Wagers"),
            CompetitionType::League { .. } => write!(f, "Leagues"),
            CompetitionType::Tournament { .. } => write!(f, "Tournaments"),
        }
    }
}

#[cw_serde]
pub struct EnrollmentEntryResponse {
    pub category_id: Option<Uint128>,
    pub id: Uint128,
    pub current_members: Uint64,
    pub min_members: Option<Uint64>,
    pub max_members: Uint64,
    pub entry_fee: Option<Coin>,
    pub duration_before: u64,
    pub has_finalized: bool,
    pub competition_info: CompetitionInfoResponse,
    pub competition_type: CompetitionType,
    pub host: Addr,
    pub competition_module: Addr,
    pub required_team_size: Option<u32>,
}

#[cw_serde]
pub struct CompetitionInfoResponse {
    pub name: String,
    pub description: String,
    pub date: Timestamp,
    pub duration: u64,
    pub rules: Option<Vec<String>>,
    pub rulesets: Option<Vec<Uint128>>,
    pub banner: Option<String>,
    pub additional_layered_fees: Option<Vec<FeeInformation<Addr>>>,
    pub competition_id: Option<Uint128>,
    pub escrow: Addr,
    pub group_contract: Addr,
}

// Core Competition

#[cw_serde]
#[derive(Default)]
pub struct CoreCompetitionsResponse {
    pub wagers: Vec<WagerResponse>,
    pub leagues: Vec<LeagueResponse>,
    pub tournaments: Vec<TournamentResponse>,
    pub enrollments: Vec<EnrollmentEntryResponse>,
}
