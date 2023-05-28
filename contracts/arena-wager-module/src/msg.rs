use cosmwasm_std::Empty;
use cw_competition::msg::{ExecuteBase, InstantiateBase, QueryBase};

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<Empty, Empty>;
pub type QueryMsg = QueryBase<Empty, Empty>;
