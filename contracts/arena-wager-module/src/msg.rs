pub use arena_interface::competition::types::WagerExt;
use arena_interface::competition::{
    msg::{ExecuteBase, InstantiateBase, MigrateBase, QueryBase, ToCompetitionExt},
    state::Competition,
    types::APIProcessing,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Empty, Uint128};

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteExt {
    ProcessCompetitionAPI {
        competition_id: Uint128,
        result: serde_json::Value,
    },
}

impl From<ExecuteExt> for ExecuteMsg {
    fn from(msg: ExecuteExt) -> Self {
        ExecuteMsg::Extension { msg }
    }
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryExt {}

impl From<QueryExt> for QueryMsg {
    fn from(msg: QueryExt) -> Self {
        QueryMsg::QueryExtension { msg }
    }
}

#[cw_serde]
#[serde(untagged)]
pub enum MigrateMsg {
    Base(MigrateBase),
}

#[cw_serde]
pub struct WagerInstantiateExt {
    pub api_processing: Option<APIProcessing>,
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, WagerInstantiateExt>;
pub type QueryMsg = QueryBase<Empty, QueryExt, WagerExt>;
pub type Wager = Competition<WagerExt>;

impl ToCompetitionExt<WagerExt> for WagerInstantiateExt {
    fn to_competition_ext(
        &self,
        _deps: cosmwasm_std::Deps,
        _group_contract: &cosmwasm_std::Addr,
    ) -> cosmwasm_std::StdResult<WagerExt> {
        Ok(WagerExt {
            api_processing: self.api_processing.clone(),
        })
    }
}
