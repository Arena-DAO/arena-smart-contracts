use std::fmt;

use cosmwasm_std::{Attribute, Decimal, StdError, StdResult};

use crate::state::{ApplicationStatus, VestingConfiguration};

impl VestingConfiguration {
    pub fn into_checked(&self) -> StdResult<()> {
        // Validate upfront_ratio
        if self.upfront_ratio > Decimal::one() || self.upfront_ratio == Decimal::zero() {
            return Err(StdError::generic_err(
                "Upfront ratio must be between 0 and 1",
            ));
        }

        // Validate vesting_time
        if self.vesting_time == 0 {
            return Err(StdError::generic_err("Vesting time must be greater than 0"));
        }

        // All checks passed
        Ok(())
    }

    pub fn into_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new("upfront_ratio", self.upfront_ratio.to_string()),
            Attribute::new("vesting_time", self.vesting_time.to_string()),
            Attribute::new("denom", self.denom.clone()),
        ]
    }
}

impl fmt::Display for ApplicationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationStatus::Pending {} => write!(f, "pending"),
            ApplicationStatus::Accepted {} => write!(f, "accepted"),
            ApplicationStatus::Rejected { .. } => write!(f, "rejected"),
        }
    }
}
