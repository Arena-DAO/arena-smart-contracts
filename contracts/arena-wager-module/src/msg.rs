use cosmwasm_schema::cw_serde;
use cosmwasm_std::Empty;
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase, ToCompetitionExt},
    state::{Competition, CompetitionResponse},
};

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<Empty, EmptyWrapper>;
pub type QueryMsg = QueryBase<Empty, Empty, Empty>;
pub type Wager = Competition<Empty>;
pub type WagerResponse = CompetitionResponse<Empty>;

#[cw_serde]
pub struct EmptyWrapper(Empty);
impl EmptyWrapper {
    pub fn new() -> Self {
        EmptyWrapper(Empty {})
    }
}
impl Default for EmptyWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ToCompetitionExt<Empty> for EmptyWrapper {
    fn to_competition_ext(&self, _deps: cosmwasm_std::Deps) -> cosmwasm_std::StdResult<Empty> {
        Ok(Empty {})
    }
}
