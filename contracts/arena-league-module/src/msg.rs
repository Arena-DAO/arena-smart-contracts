use crate::state::{LeagueExt, Match, MatchResult, PointAdjustment};
use arena_interface::competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase, ToCompetitionExt},
    state::{Competition, CompetitionResponse},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Empty, Int128, StdError, StdResult, Uint128, Uint64};
use itertools::Itertools;

#[cw_serde]
pub enum ExecuteExt {
    ProcessMatch {
        league_id: Uint128,
        round_number: Uint64,
        match_results: Vec<MatchResultMsg>,
    },
    UpdateDistribution {
        league_id: Uint128,
        distribution: Vec<Decimal>,
    },
    AddPointAdjustments {
        league_id: Uint128,
        addr: String,
        point_adjustments: Vec<PointAdjustment>,
    },
}

#[cw_serde]
pub struct MatchResultMsg {
    pub match_number: Uint128,
    pub match_result: MatchResult,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum LeagueQueryExt {
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
    #[returns(Vec<PointAdjustmentResponse>)]
    PointAdjustments {
        league_id: Uint128,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(DumpStateResponse)]
    DumpState {
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
pub struct LeagueInstantiateExt {
    pub match_win_points: Uint64,
    pub match_draw_points: Uint64,
    pub match_lose_points: Uint64,
    pub teams: Vec<String>,
    pub distribution: Vec<Decimal>,
}

impl ToCompetitionExt<LeagueExt> for LeagueInstantiateExt {
    fn to_competition_ext(&self, _deps: cosmwasm_std::Deps) -> StdResult<LeagueExt> {
        let team_count = self.teams.len();
        if team_count < 2 {
            return Err(StdError::GenericErr {
                msg: "At least 2 teams should be provided".to_string(),
            });
        }
        if self.teams.iter().unique().count() != team_count {
            return Err(StdError::GenericErr {
                msg: "Teams should not contain duplicates".to_string(),
            });
        }
        if self.distribution.len() > team_count {
            return Err(StdError::GenericErr {
                msg: "Cannot have a distribution size bigger than the teams size".to_string(),
            });
        }
        if self.distribution.iter().sum::<Decimal>() != Decimal::one() {
            return Err(StdError::generic_err("The distribution must sum up to 1"));
        }

        let matches = team_count * (team_count - 1) / 2;
        let rounds = if team_count % 2 == 0 {
            team_count - 1
        } else {
            team_count
        };

        Ok(LeagueExt {
            match_win_points: self.match_win_points,
            match_draw_points: self.match_draw_points,
            match_lose_points: self.match_lose_points,
            teams: Uint64::from(team_count as u64),
            rounds: Uint64::from(rounds as u64),
            matches: Uint128::from(matches as u128),
            processed_matches: Uint128::zero(),
            distribution: self.distribution.clone(),
        })
    }
}

#[cw_serde]
pub struct MemberPoints {
    pub member: Addr,
    pub points: Int128,
    pub matches_played: Uint64,
}

#[cw_serde]
pub struct RoundResponse {
    pub round_number: Uint64,
    pub matches: Vec<Match>,
}

#[cw_serde]
pub struct PointAdjustmentResponse {
    pub addr: Addr,
    pub point_adjustments: Vec<PointAdjustment>,
}

#[cw_serde]
pub struct DumpStateResponse {
    pub leaderboard: Vec<MemberPoints>,
    pub round: RoundResponse,
    pub point_adjustments: Vec<PointAdjustmentResponse>,
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, LeagueInstantiateExt>;
pub type QueryMsg = QueryBase<Empty, LeagueQueryExt, LeagueExt>;
pub type League = Competition<LeagueExt>;
pub type LeagueResponse = CompetitionResponse<LeagueExt>;
