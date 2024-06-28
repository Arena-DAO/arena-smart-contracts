use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Binary, Decimal, Deps, StdError, StdResult};
use cw_address_like::AddressLike;

#[cw_serde]
pub struct FeeInformation<T: AddressLike> {
    pub tax: Decimal,
    pub receiver: T,
    pub cw20_msg: Option<Binary>,
    pub cw721_msg: Option<Binary>,
}

impl FeeInformation<String> {
    pub fn into_checked(&self, deps: Deps) -> StdResult<FeeInformation<Addr>> {
        ensure!(
            self.tax < Decimal::one(),
            StdError::generic_err("Tax must be less than 100%")
        );

        Ok(FeeInformation {
            receiver: deps.api.addr_validate(&self.receiver)?,
            tax: self.tax,
            cw20_msg: self.cw20_msg.clone(),
            cw721_msg: self.cw721_msg.clone(),
        })
    }
}
