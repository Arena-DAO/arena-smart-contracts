use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult};

#[cw_serde]
pub struct Cw721CollectionVerified {
    pub addr: Addr,
    pub token_ids: Vec<String>,
}

#[cw_serde]
pub struct Cw721Collection {
    pub addr: String,
    pub token_ids: Vec<String>,
}

impl Cw721Collection {
    pub fn to_validated(self, deps: Deps) -> StdResult<Cw721CollectionVerified> {
        Ok(Cw721CollectionVerified {
            addr: deps.api.addr_validate(&self.addr)?,
            token_ids: self.token_ids,
        })
    }
}
