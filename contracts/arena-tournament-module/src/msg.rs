use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_controllers::AdminResponse;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateAdmin { admin: Option<String> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AdminResponse)]
    Admin {},
}

#[cw_serde]
pub struct MigrateMsg {}
