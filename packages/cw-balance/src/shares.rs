use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult, Uint128};
use cw_address_like::AddressLike;

#[cw_serde]
pub struct MemberShare<T: AddressLike> {
    pub addr: T,
    pub shares: Uint128,
}

impl MemberShare<String> {
    pub fn to_validated(&self, deps: Deps) -> StdResult<MemberShare<Addr>> {
        Ok(MemberShare {
            addr: deps.api.addr_validate(&self.addr)?,
            shares: self.shares,
        })
    }
}
