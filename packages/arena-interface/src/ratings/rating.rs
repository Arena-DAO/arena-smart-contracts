use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Decimal};

#[cw_serde]
pub struct Rating {
    pub value: Decimal,
    pub phi: Decimal,
    pub sigma: Decimal,
    pub last_block: Option<BlockInfo>,
}

impl Rating {
    pub fn new(value: Decimal, phi: Decimal, sigma: Decimal) -> Self {
        Self {
            value,
            phi,
            sigma,
            last_block: None,
        }
    }
}

impl Default for Rating {
    fn default() -> Self {
        Self {
            value: Decimal::from_ratio(1500u128, 1u128),
            phi: Decimal::from_ratio(300u128, 1u128),
            sigma: Decimal::from_ratio(6u128, 100u128),
            last_block: None,
        }
    }
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rating(value: {}, phi: {}, sigma: {})",
            self.value, self.phi, self.sigma
        )?;

        if let Some(block_info) = &self.last_block {
            write!(
                f,
                ", last_block: {{ height: {}, time: {} }}",
                block_info.height, block_info.time
            )
        } else {
            write!(f, ", last_block: None")
        }
    }
}
