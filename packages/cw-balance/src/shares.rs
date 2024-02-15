use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Deps, StdResult};
use cw_address_like::AddressLike;

#[cw_serde]
pub struct MemberPercentage<T: AddressLike> {
    pub addr: T,
    pub percentage: Decimal,
}

impl MemberPercentage<String> {
    pub fn into_checked(&self, deps: Deps) -> StdResult<MemberPercentage<Addr>> {
        Ok(MemberPercentage {
            addr: deps.api.addr_validate(&self.addr)?,
            percentage: self.percentage,
        })
    }
}
