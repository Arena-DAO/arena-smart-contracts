use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Decimal};

use crate::core::TaxConfigurationResponse;

#[cw_serde]
pub struct TaxConfiguration {
    pub cw20_msg: Option<Binary>,
    pub cw721_msg: Option<Binary>,
}

impl TaxConfiguration {
    pub fn into_response(self, tax: Decimal) -> TaxConfigurationResponse {
        TaxConfigurationResponse {
            tax,
            cw20_msg: self.cw20_msg,
            cw721_msg: self.cw721_msg,
        }
    }
}
