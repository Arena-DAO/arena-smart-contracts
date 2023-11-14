use crate::state::Result;
#[allow(unused_imports)]
use crate::state::RoundResponse;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Empty, Uint128, Uint64};
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase},
    state::{Competition, CompetitionResponse},
};
use cw_utils::Duration;

#[cw_serde]
pub enum ExecuteExt {
    ProcessMatch {
        league_id: Uint128,
        round_number: Uint64,
        match_results: Vec<MatchResult>,
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
    Leaderboard { league_id: Uint128 },
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

#[cw_serde]
pub struct CompetitionExt {
    pub match_win_points: Uint128,
    pub match_draw_points: Uint128,
    pub match_lose_points: Uint128,
    pub rounds: Uint64,
}

#[cw_serde]
pub struct CompetitionInstantiateExt {
    pub match_win_points: Uint128,
    pub match_draw_points: Uint128,
    pub match_lose_points: Uint128,
    pub teams: Vec<String>,
    pub round_duration: Duration,
}

impl From<CompetitionInstantiateExt> for CompetitionExt {
    fn from(value: CompetitionInstantiateExt) -> Self {
        CompetitionExt {
            match_win_points: value.match_win_points,
            match_draw_points: value.match_draw_points,
            match_lose_points: value.match_lose_points,
            rounds: Uint64::zero(),
        }
    }
}

#[cw_serde]
pub struct MemberPoints {
    pub member: Addr,
    pub points: Uint128,
    pub matches_played: Uint64,
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, CompetitionInstantiateExt>;
pub type QueryMsg = QueryBase<QueryExt, CompetitionExt>;
pub type League = Competition<CompetitionExt>;
pub type LeagueResponse = CompetitionResponse<CompetitionExt>;
