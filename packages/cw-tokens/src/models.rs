use crate::GenericTokenBalance;
use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct GenericBalanceResponse {
    pub balance: Vec<GenericTokenBalance>,
}
