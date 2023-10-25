use cosmwasm_schema::cw_serde;
use cosmwasm_std::Empty;
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase},
    state::{Competition, CompetitionResponse},
};

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<Empty, Empty>;
pub type QueryMsg = QueryBase<Empty, Empty>;
pub type Wager = Competition<Empty>;
pub type WagerResponse = CompetitionResponse<Empty>;
