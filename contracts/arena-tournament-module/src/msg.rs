use crate::state::{EliminationType, MatchResult, TournamentExt};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Empty, StdError, StdResult, Uint128};
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase, ToCompetitionExt},
    state::{Competition, CompetitionResponse},
};
use itertools::Itertools;

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
#[impl_into(ExecuteMsg)]
pub enum ExecuteExt {
    ProcessMatch {
        tournament_id: Uint128,
        match_results: Vec<MatchResultMsg>,
    },
}

impl From<ExecuteExt> for ExecuteMsg {
    fn from(msg: ExecuteExt) -> Self {
        ExecuteMsg::Extension { msg }
    }
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
#[impl_into(QueryMsg)]
pub enum QueryExt {
    #[returns(Vec<crate::state::Match>)]
    Bracket {
        tournament_id: Uint128,
        start_after: Option<Uint128>,
    },
    #[returns(crate::state::Match)]
    r#Match {
        tournament_id: Uint128,
        match_number: Uint128,
    },
}

impl From<QueryExt> for QueryMsg {
    fn from(msg: QueryExt) -> Self {
        QueryMsg::QueryExtension { msg }
    }
}

#[cw_serde]
pub struct MatchResultMsg {
    pub match_number: Uint128,
    pub match_result: MatchResult,
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

/// This is used to completely generate schema types
/// QueryExt response types are hidden by the QueryBase mapping to Binary output
#[cw_serde]
pub struct SudoMsg {
    pub matches: Vec<crate::state::Match>,
}

#[cw_serde]
pub struct TournamentInstantiateExt {
    pub elimination_type: EliminationType, // Enum for single or double elimination
    pub teams: Vec<String>,                // List of team addresses ordered by seeding
    pub distribution: Vec<Decimal>,
}

impl ToCompetitionExt<TournamentExt> for TournamentInstantiateExt {
    fn to_competition_ext(&self, _deps: cosmwasm_std::Deps) -> StdResult<TournamentExt> {
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

        let max_placements = match self.elimination_type {
            EliminationType::SingleElimination {
                play_third_place_match,
            } => {
                if play_third_place_match {
                    if team_count < 4 {
                        return Err(StdError::GenericErr {
                            msg: "At least 4 teams should be provided for a 3rd place match"
                                .to_string(),
                        });
                    }

                    usize::min(team_count, 4)
                } else {
                    2
                }
            }
            EliminationType::DoubleElimination => usize::min(team_count, 3),
        };

        if self.distribution.len() > max_placements {
            return Err(StdError::GenericErr {
                msg: "Cannot have a distribution size bigger than the possible placements"
                    .to_string(),
            });
        }
        if self.distribution.iter().sum::<Decimal>() != Decimal::one() {
            return Err(StdError::generic_err("The distribution must sum up to 1"));
        }

        let total_matches = if let EliminationType::SingleElimination {
            play_third_place_match,
        } = self.elimination_type
        {
            (team_count - 1) + if play_third_place_match { 1 } else { 0 }
        } else {
            2 * (team_count - 1) // + R (rebuttal match)
        };

        Ok(TournamentExt {
            distribution: self.distribution.clone(),
            elimination_type: self.elimination_type.clone(),
            total_matches: Uint128::from(total_matches as u128),
            processed_matches: Uint128::zero(),
        })
    }
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, TournamentInstantiateExt>;
pub type QueryMsg = QueryBase<Empty, QueryExt, TournamentExt>;
pub type Tournament = Competition<TournamentExt>;
pub type TournamentResponse = CompetitionResponse<TournamentExt>;
