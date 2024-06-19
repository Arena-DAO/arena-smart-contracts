use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
use cw_utils::Expiration;

use crate::state::CompetitionInfo;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_ownable::cw_ownable_execute]
#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    CreateCompetition {
        /// Override the minimum members for the competition
        min_members: Option<Uint128>,
        max_members: Uint128,
        /// The entry fee of the competition
        entry_fee: Option<Coin>,
        expiration: Expiration,
        category_id: Option<Uint128>,
        competition_info: CompetitionInfo<String>,
        /// Is the creator a member on creation
        /// Defaults to false
        is_creator_member: Option<bool>,
    },
}

#[cw_ownable::cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}
