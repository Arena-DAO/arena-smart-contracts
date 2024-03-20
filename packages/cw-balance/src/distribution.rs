use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Deps, StdError, StdResult};
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
