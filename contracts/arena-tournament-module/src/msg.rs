use crate::state::{EliminationType, MatchResult, TournamentExt};
use arena_interface::{
    competition::{
        msg::{ExecuteBase, InstantiateBase, QueryBase, ToCompetitionExt},
        state::{Competition, CompetitionResponse},
    },
    group,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Empty, StdError, StdResult, Uint128, Uint64};

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteExt {
    ProcessMatch {
        tournament_id: Uint128,
        match_results: Vec<MatchResultMsg>,
    },
    InstantiateTournament {},
}

impl From<ExecuteExt> for ExecuteMsg {
    fn from(msg: ExecuteExt) -> Self {
        ExecuteMsg::Extension { msg }
    }
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
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
    pub distribution: Vec<Decimal>,
}

impl ToCompetitionExt<TournamentExt> for TournamentInstantiateExt {
    fn to_competition_ext(
        &self,
        deps: cosmwasm_std::Deps,
        group_contract: &Addr,
    ) -> StdResult<TournamentExt> {
        let team_count: Uint64 = deps.querier.query_wasm_smart(
            group_contract.to_string(),
            &group::QueryMsg::MembersCount {},
        )?;

        if team_count < Uint64::new(2) {
            return Err(StdError::GenericErr {
                msg: "At least 2 teams should be provided".to_string(),
            });
        }

        let max_placements = match self.elimination_type {
            EliminationType::SingleElimination {
                play_third_place_match,
            } => {
                if play_third_place_match {
                    if team_count < Uint64::new(4) {
                        return Err(StdError::GenericErr {
                            msg: "At least 4 teams should be provided for a 3rd place match"
                                .to_string(),
                        });
                    }

                    Uint64::min(team_count, Uint64::new(4))
                } else {
                    Uint64::new(2)
                }
            }
            EliminationType::DoubleElimination => Uint64::min(team_count, Uint64::new(3)),
        };

        if Uint64::new(self.distribution.len() as u64) > max_placements {
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
            (team_count - Uint64::one())
                + if play_third_place_match {
                    Uint64::one()
                } else {
                    Uint64::zero()
                }
        } else {
            Uint64::new(2) * (team_count - Uint64::one()) // + R (rebuttal match)
        };

        Ok(TournamentExt {
            distribution: self.distribution.clone(),
            elimination_type: self.elimination_type.clone(),
            total_matches: total_matches.into(),
            processed_matches: Uint128::zero(),
        })
    }
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, TournamentInstantiateExt>;
pub type QueryMsg = QueryBase<Empty, QueryExt, TournamentExt>;
pub type Tournament = Competition<TournamentExt>;
pub type TournamentResponse = CompetitionResponse<TournamentExt>;
