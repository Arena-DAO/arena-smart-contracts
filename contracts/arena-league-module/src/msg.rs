#[allow(unused_imports)]
use crate::state::RoundResponse;
use crate::state::{Result, TournamentExt};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, StdResult, Uint128, Uint64};
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase, ToCompetitionExt},
    state::{Competition, CompetitionResponse},
};

#[cw_serde]
pub enum ExecuteExt {
    ProcessMatch {
        league_id: Uint128,
        round_number: Uint64,
        match_results: Vec<MatchResult>,
    },
    UpdateDistribution {
        league_id: Uint128,
        distribution: Vec<Decimal>,
    },
}

#[cw_serde]
pub struct MatchResult {
    pub match_number: Uint128,
    pub result: Option<Result>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    #[returns(Vec<MemberPoints>)]
    Leaderboard {
        league_id: Uint128,
        round: Option<Uint64>,
    },
    #[returns(RoundResponse)]
    Round {
        league_id: Uint128,
        round_number: Uint64,
    },
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

/// This is used to completely generate schema types
/// QueryExt response types are hidden by the QueryBase mapping to Binary output
#[cw_serde]
pub struct SudoMsg {
    pub member_points: MemberPoints,
    pub round_response: RoundResponse,
}

#[cw_serde]
pub struct CompetitionExt {
    pub match_win_points: Uint128,
    pub match_draw_points: Uint128,
    pub match_lose_points: Uint128,
    pub rounds: Uint64,
    pub matches: Uint128,
    pub teams: Uint64,
    pub processed_matches: Uint128,
    pub distribution: Vec<Decimal>,
}

#[cw_serde]
pub struct CompetitionInstantiateExt {
    pub match_win_points: Uint128,
    pub match_draw_points: Uint128,
    pub match_lose_points: Uint128,
    pub teams: Vec<String>,
    pub distribution: Vec<Decimal>,
}

impl ToCompetitionExt<CompetitionExt> for CompetitionInstantiateExt {
    fn to_competition_ext(&self, _deps: cosmwasm_std::Deps) -> StdResult<CompetitionExt> {
        Ok(CompetitionExt {
            match_win_points: self.match_win_points,
            match_draw_points: self.match_draw_points,
            match_lose_points: self.match_lose_points,
            teams: Uint64::zero(),
            rounds: Uint64::zero(),
            matches: Uint128::zero(),
            processed_matches: Uint128::zero(),
            distribution: self.distribution.clone(),
        })
    }
}

#[cw_serde]
pub struct MemberPoints {
    pub member: Addr,
    pub points: Uint128,
    pub matches_played: Uint64,
}

pub type InstantiateMsg = InstantiateBase<TournamentExt>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, CompetitionInstantiateExt>;
pub type QueryMsg = QueryBase<TournamentExt, QueryExt, CompetitionExt>;
pub type League = Competition<CompetitionExt>;
pub type LeagueResponse = CompetitionResponse<CompetitionExt>;
