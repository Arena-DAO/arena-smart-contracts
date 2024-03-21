use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_string, Addr, Decimal, Deps, StdError, StdResult};
use cw_address_like::AddressLike;
use itertools::Itertools;
use serde::Serialize;

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
        let total_weight = self
            .member_percentages
            .iter()
            .try_fold(Decimal::zero(), |accumulator, x| {
                accumulator.checked_add(x.percentage)
            })?;

        if total_weight != Decimal::one() {
            return Err(StdError::generic_err("Total weight is not equal to 1"));
        }

        if self.member_percentages.is_empty() {
            return Err(StdError::generic_err("Member percentages cannot be empty"));
        }

        let unique_members = self
            .member_percentages
            .iter()
            .unique_by(|x| &x.addr)
            .count();

        if unique_members != self.member_percentages.len() {
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
