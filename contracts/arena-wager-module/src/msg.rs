use arena_interface::competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase, ToCompetitionExt},
    state::{Competition, CompetitionResponse},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Empty;

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteExt {}

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
pub enum MigrateMsg {
    FromCompatible {},
}

#[cw_serde]
pub struct WagerInstantiateExt {}

#[cw_serde]
pub struct WagerExt {}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ExecuteExt, WagerInstantiateExt>;
pub type QueryMsg = QueryBase<Empty, QueryExt, WagerExt>;
pub type Wager = Competition<WagerExt>;
pub type WagerResponse = CompetitionResponse<WagerExt>;

impl ToCompetitionExt<WagerExt> for WagerInstantiateExt {
    fn to_competition_ext(
        &self,
        _deps: cosmwasm_std::Deps,
        _group_contract: &cosmwasm_std::Addr,
    ) -> cosmwasm_std::StdResult<WagerExt> {
        Ok(WagerExt {})
    }
}
