use crate::state::Round;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64};
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase},
    state::{Competition, CompetitionResponse},
};
use cw_utils::Duration;
use dao_interface::state::ModuleInstantiateInfo;

#[cw_serde]
pub struct InstantiateExt {
    pub wagers_key: String,
}

#[cw_serde]
pub enum ExecuteExt {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    #[returns(Vec<MemberPoints>)]
    Leaderboard { league_id: Uint128 },
    #[returns(Round)]
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
    pub wager_module: Addr,
}

#[cw_serde]
pub struct CompetitionInstantiateExt {
    pub match_win_points: Uint128,
    pub match_draw_points: Uint128,
    pub match_lose_points: Uint128,
    pub teams: Vec<String>,
    pub round_duration: Duration,
    pub wager_dao: ModuleInstantiateInfo,
    pub wager_name: String,
    pub wager_description: String,
}

impl From<CompetitionInstantiateExt> for CompetitionExt {
    fn from(value: CompetitionInstantiateExt) -> Self {
        CompetitionExt {
            match_win_points: value.match_win_points,
            match_draw_points: value.match_draw_points,
            match_lose_points: value.match_lose_points,
            rounds: Uint64::zero(),
            wager_module: Addr::unchecked("default"),
        }
    }
}

#[cw_serde]
pub struct MemberPoints {
    pub member: Addr,
    pub points: Uint128,
    pub matches_played: Uint64,
}

pub type InstantiateMsg = InstantiateBase<InstantiateExt>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, CompetitionInstantiateExt>;
pub type QueryMsg = QueryBase<QueryExt, CompetitionExt>;
pub type League = Competition<CompetitionExt>;
pub type LeagueResponse = CompetitionResponse<CompetitionExt>;
