use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult};

#[cw_serde]
pub struct Cw721CollectionVerified {
    pub address: Addr,
    pub token_ids: Vec<String>,
}

#[cw_serde]
pub struct Cw721Collection {
    pub address: String,
    pub token_ids: Vec<String>,
}

impl Cw721Collection {
    pub fn into_checked(self, deps: Deps) -> StdResult<Cw721CollectionVerified> {
        Ok(Cw721CollectionVerified {
            address: deps.api.addr_validate(&self.address)?,
            token_ids: self.token_ids,
        })
    }
}

impl fmt::Display for Cw721Collection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "address: {}, token_ids: {}",
            self.address,
            self.token_ids.join(",")
        )
    }
}

impl fmt::Display for Cw721CollectionVerified {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "address: {}, token_ids: {}",
            self.address,
            self.token_ids.join(",")
        )
    }
}
