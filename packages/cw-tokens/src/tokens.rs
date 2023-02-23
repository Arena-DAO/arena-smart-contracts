use crate::BalanceError;
use core::fmt;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Deps, StdResult, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use std::hash::{Hash, Hasher};

pub trait CoinExtensions {
    fn to_generic(&self) -> GenericTokenBalance;
}

pub trait BatchCoinExtensions {
    fn to_generic_batch(&self) -> Vec<GenericTokenBalance>;
}

pub trait TokenExtensions {
    //addr is the token's address
    fn to_generic(&self, addr: &Addr) -> GenericTokenBalance;
}

impl TokenExtensions for Cw721ReceiveMsg {
    fn to_generic(&self, addr: &Addr) -> GenericTokenBalance {
        GenericTokenBalance {
            addr: Some(addr.clone()),
            denom: Some(self.token_id.clone()),
            amount: Uint128::one(),
            token_type: GenericTokenType::Cw721,
        }
    }
}

impl TokenExtensions for Cw20ReceiveMsg {
    fn to_generic(&self, addr: &Addr) -> GenericTokenBalance {
        GenericTokenBalance {
            addr: Some(addr.clone()),
            denom: None,
            amount: self.amount,
            token_type: GenericTokenType::Cw20,
        }
    }
}

impl CoinExtensions for Coin {
    fn to_generic(&self) -> GenericTokenBalance {
        GenericTokenBalance {
            addr: None,
            denom: Some(self.denom.clone()),
            amount: self.amount,
            token_type: GenericTokenType::Native,
        }
    }
}

impl BatchCoinExtensions for Vec<Coin> {
    fn to_generic_batch(&self) -> Vec<GenericTokenBalance> {
        self.iter().map(|x| x.to_generic()).collect()
    }
}

#[cw_serde]
pub enum GenericTokenType {
    Native,
    Cw20,
    Cw721,
}

#[cw_serde]
pub struct GenericTokenBalanceRaw {
    pub addr: Option<String>,
    pub denom: Option<String>,
    pub amount: Uint128,
    pub token_type: GenericTokenType,
}

impl GenericTokenBalanceRaw {
    fn to_validated(&self, deps: Deps) -> StdResult<GenericTokenBalance> {
        Ok(GenericTokenBalance {
            addr: self
                .addr
                .clone()
                .map(|x| deps.api.addr_validate(&x))
                .transpose()?,
            denom: self.denom.clone(),
            amount: self.amount,
            token_type: self.token_type.clone(),
        })
    }
}

#[cw_serde]
pub struct GenericTokenBalance {
    pub addr: Option<Addr>,
    pub denom: Option<String>,
    pub amount: Uint128,
    pub token_type: GenericTokenType,
}

impl fmt::Display for GenericTokenBalance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({}, {}) - {}",
            self.addr.clone().map(|x| x.to_string()).unwrap_or_default(),
            self.denom.clone().unwrap_or_default(),
            self.amount
        )
    }
}

impl Hash for GenericTokenBalance {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
        self.denom.hash(state);
    }
}

impl Eq for GenericTokenBalance {}

impl GenericTokenBalance {
    pub fn clone_with_amount(&self, amount: Uint128) -> GenericTokenBalance {
        GenericTokenBalance {
            addr: self.addr.clone(),
            denom: self.denom.clone(),
            amount,
            token_type: self.token_type.clone(),
        }
    }

    pub fn get_token_id(&self) -> (Option<Addr>, Option<String>) {
        (self.addr.clone(), self.denom.clone())
    }

    fn guard_valid_arithmetic(&self, other: &Self) -> Result<(), BalanceError> {
        if self.token_type != other.token_type {
            return Err(BalanceError::MismatchedTypes {
                type1: self.token_type.clone(),
                type2: other.token_type.clone(),
            });
        }
        if self != other {
            return Err(BalanceError::MismatchedToken {
                token1: self.get_token_id(),
                token2: other.get_token_id(),
            });
        }

        Ok(())
    }

    pub fn checked_add(&self, other: &Self) -> Result<Self, BalanceError> {
        self.guard_valid_arithmetic(&other)?;

        let amount = self.amount.checked_add(other.amount)?;

        Ok(self.clone_with_amount(amount))
    }

    pub fn checked_sub(&self, other: &Self) -> Result<Self, BalanceError> {
        self.guard_valid_arithmetic(&other)?;

        let amount = self.amount.checked_sub(other.amount)?;

        Ok(self.clone_with_amount(amount))
    }
}
