use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_string, Addr, Decimal, Deps, StdError, StdResult};
use cw_address_like::AddressLike;
use serde::Serialize;
use std::fmt::Display;

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

#[cw_serde]
pub struct Distribution<T: AddressLike> {
    pub member_percentages: Vec<MemberPercentage<T>>,
    pub remainder_addr: T,
}

impl Distribution<String> {
    pub fn into_checked(&self, deps: Deps) -> StdResult<Distribution<Addr>> {
        if self.member_percentages.is_empty() {
            return Err(StdError::generic_err("Member percentages cannot be empty"));
        }

        let (total_weight, unique_members) = self.member_percentages.iter().fold(
            (Decimal::zero(), std::collections::HashSet::new()),
            |(weight, mut set), x| {
                (weight + x.percentage, {
                    set.insert(&x.addr);
                    set
                })
            },
        );

        if total_weight != Decimal::one() {
            return Err(StdError::generic_err("Total weight must be equal to 1"));
        }

        if unique_members.len() != self.member_percentages.len() {
            return Err(StdError::generic_err("All members must be unique"));
        }

        Ok(Distribution::<Addr> {
            member_percentages: self
                .member_percentages
                .iter()
                .map(|x| x.into_checked(deps))
                .collect::<StdResult<_>>()?,
            remainder_addr: deps.api.addr_validate(&self.remainder_addr)?,
        })
    }
}

impl<T: AddressLike + Serialize> Display for Distribution<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match to_json_string(self) {
            Ok(json) => write!(f, "{}", json),
            Err(e) => write!(f, "Serialization Error: {}", e),
        }
    }
}

impl Distribution<Addr> {
    pub fn into_unchecked(&self) -> Distribution<String> {
        Distribution::<String> {
            member_percentages: self
                .member_percentages
                .iter()
                .map(|x| MemberPercentage::<String> {
                    addr: x.addr.to_string(),
                    percentage: x.percentage,
                })
                .collect(),
            remainder_addr: self.remainder_addr.to_string(),
        }
    }
}
