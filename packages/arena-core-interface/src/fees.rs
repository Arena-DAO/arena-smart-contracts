use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Decimal, Deps, StdResult};
use cw_address_like::AddressLike;

use crate::msg::TaxConfigurationResponse;

#[cw_serde]
pub struct FeeInformation<T: AddressLike> {
    pub tax: Decimal,
    pub receiver: T,
    pub cw20_msg: Option<Binary>,
    pub cw721_msg: Option<Binary>,
}

impl FeeInformation<String> {
    pub fn into_checked(&self, deps: Deps) -> StdResult<FeeInformation<Addr>> {
        Ok(FeeInformation {
            receiver: deps.api.addr_validate(&self.receiver)?,
            tax: self.tax,
            cw20_msg: self.cw20_msg.clone(),
            cw721_msg: self.cw721_msg.clone(),
        })
    }
}

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
