use arena_interface::competition::migrate::IntoCompetitionExt;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Map;

#[cw_serde]
pub struct Match {
    pub match_number: Uint128,
    pub team_1: Option<Addr>,
    pub team_2: Option<Addr>,
    pub result: Option<MatchResult>,        // Result as an enum
    pub next_match_winner: Option<Uint128>, // Next match for the winner
    pub next_match_loser: Option<Uint128>,  // Next match for the loser (used in double elimination)
    pub is_losers_bracket: Option<bool>, // Is match a part of the loser's bracket (used in double elimination)
}

#[cw_serde]
pub enum MatchResult {
    Team1,
    Team2,
}

#[cw_serde]
pub struct TournamentExt {
    pub elimination_type: EliminationType, // Enum for single or double elimination
    pub distribution: Vec<Decimal>,
    pub total_matches: Uint128,
    pub processed_matches: Uint128,
}

impl IntoCompetitionExt<TournamentExt> for TournamentExt {
    fn into_competition_ext(self) -> TournamentExt {
        self
    }
}

#[cw_serde]
pub enum EliminationType {
    SingleElimination { play_third_place_match: bool },
    DoubleElimination,
}

/// (Tournament Id, Match Number)
pub const MATCHES: Map<(u128, u128), Match> = Map::new("tournament_matches");
