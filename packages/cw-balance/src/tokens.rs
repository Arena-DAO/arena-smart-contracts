use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult};

#[cw_serde]
pub struct Cw721TokensVerified {
    pub addr: Addr,
    pub token_ids: Vec<String>,
}

#[cw_serde]
pub struct Cw721Tokens {
    pub addr: String,
    pub tokens: Vec<String>,
}

impl Cw721Tokens {
    pub fn to_validated(&self, deps: Deps) -> StdResult<Cw721TokensVerified> {
        Ok(Cw721TokensVerified {
            addr: deps.api.addr_validate(&self.addr)?,
            token_ids: self.tokens.clone(),
        })
    }
}
