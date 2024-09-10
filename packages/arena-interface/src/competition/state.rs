use crate::fees::FeeInformation;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Decimal, StdError, StdResult, Timestamp, Uint128};
use cw_utils::Expiration;
use std::fmt;

use super::msg::StatAggregationType;

#[cw_serde]
#[derive(Default)]
pub enum CompetitionStatus {
    Pending,
    Active {
        activation_height: u64,
    },
    #[default]
    Inactive,
    Jailed {
        activation_height: u64,
    },
}

impl fmt::Display for CompetitionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompetitionStatus::Pending => write!(f, "Pending"),
            CompetitionStatus::Jailed {
                activation_height: _,
            } => write!(f, "Jailed"),
            CompetitionStatus::Active {
                activation_height: _,
            } => write!(f, "Active"),
            CompetitionStatus::Inactive => write!(f, "Inactive"),
        }
    }
}

#[cw_serde]
pub struct Competition<CompetitionExt> {
    pub id: Uint128,
    pub category_id: Option<Uint128>,
    pub admin_dao: Addr,
    pub host: Addr,
    pub escrow: Option<Addr>,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub expiration: Expiration,
    pub rulesets: Option<Vec<Uint128>>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    /// Additional layered fees
    pub fees: Option<Vec<FeeInformation<Addr>>>,
    /// A banner-image link for the competition
    pub banner: Option<String>,
}

/// CompetitionResponse extends the Competition by also returning rules, is_expired, and
#[cw_serde]
pub struct CompetitionResponse<CompetitionExt> {
    pub id: Uint128,
    pub category_id: Option<Uint128>,
    pub host: Addr,
    pub escrow: Option<Addr>,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub is_expired: bool,
    pub rules: Option<Vec<String>>,
    pub rulesets: Option<Vec<Uint128>>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    pub expiration: Expiration,
    pub fees: Option<Vec<FeeInformation<Addr>>>,
    pub banner: Option<String>,
}

impl<CompetitionExt> Competition<CompetitionExt> {
    pub fn into_response(
        self,
        rules: Option<Vec<String>>,
        block: &BlockInfo,
    ) -> CompetitionResponse<CompetitionExt> {
        let is_expired = self.expiration.is_expired(block);

        CompetitionResponse {
            id: self.id,
            category_id: self.category_id,
            host: self.host,
            escrow: self.escrow,
            name: self.name,
            description: self.description,
            start_height: self.start_height,
            is_expired,
            rules,
            rulesets: self.rulesets,
            status: self.status,
            extension: self.extension,
            expiration: self.expiration,
            fees: self.fees,
            banner: self.banner,
        }
    }
}

#[cw_serde]
pub struct Config<InstantiateExt> {
    pub key: String,
    pub description: String,
    pub extension: InstantiateExt,
}

#[cw_serde]
pub struct Evidence {
    pub id: Uint128,
    pub submit_user: Addr,
    pub content: String,
    pub submit_time: Timestamp,
}

#[cw_serde]
pub enum StatValueType {
    Bool,
    Decimal,
    Uint,
}

#[cw_serde]
pub struct StatType {
    pub name: String,
    pub value_type: StatValueType,
    pub tie_breaker_priority: Option<u8>,
    pub is_beneficial: bool,
    pub aggregation_type: Option<StatAggregationType>,
}

// Stats

#[cw_serde]
pub enum StatValue {
    Bool(bool),
    Decimal(Decimal),
    Uint(Uint128),
}

impl StatValue {
    pub fn checked_add(self, other: StatValue) -> StdResult<StatValue> {
        match (self, other) {
            (StatValue::Bool(_), _) | (_, StatValue::Bool(_)) => Err(StdError::generic_err(
                "Cannot perform arithmetic on boolean stats",
            )),
            (StatValue::Uint(a), StatValue::Uint(b)) => Ok(StatValue::Uint(a.checked_add(b)?)),
            (StatValue::Decimal(a), StatValue::Decimal(b)) => {
                Ok(StatValue::Decimal(a.checked_add(b)?))
            }
            _ => Err(StdError::generic_err("Cannot add different types of stats")),
        }
    }

    pub fn checked_div(self, divisor: Decimal) -> StdResult<Decimal> {
        match self {
            StatValue::Bool(_) => Err(StdError::generic_err("Cannot divide boolean stats")),
            StatValue::Uint(a) => Ok(Decimal::from_ratio(a, 1u128)
                .checked_div(divisor)
                .map_err(|x| StdError::generic_err(x.to_string()))?),
            StatValue::Decimal(a) => Ok(a
                .checked_div(divisor)
                .map_err(|x| StdError::generic_err(x.to_string()))?),
        }
    }
}
