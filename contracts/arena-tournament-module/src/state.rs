use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
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

/// (Tournament Id, Match Number)
pub const MATCHES: Map<(u128, u128), Match> = Map::new("tournament_matches");
