use cosmwasm_schema::cw_serde;
use cosmwasm_std::Empty;
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase},
    state::Competition,
};

#[cw_serde]
pub enum MigrateMsg {
    FromV1 {},
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<Empty, Empty>;
pub type QueryMsg = QueryBase<Empty, Empty>;
pub type Wager = Competition<Empty>;
