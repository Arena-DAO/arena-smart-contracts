use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdError, StdResult, Uint128};

#[cw_serde]
pub struct MemberStatsMsg {
    pub addr: String,
    pub stats: Vec<StatMsg>,
}

#[cw_serde]
pub struct MemberStatsRemoveMsg {
    pub addr: String,
    pub stats: Vec<StatsRemoveMsg>,
}

#[cw_serde]
pub struct StatsRemoveMsg {
    pub name: String,
    pub height: u64,
}

#[cw_serde]
#[serde(untagged)]
pub enum StatMsg {
    // Variant for inputting stats
    InputStat {
        name: String,
        value: StatValue,
    },
    // Variant for querying historical stats
    HistoricalStat {
        name: String,
        value: StatValue,
        height: u64,
    },
    // Variant for querying stats table
    StatWithAggregation {
        name: String,
        value: StatValue,
        aggregation_type: Option<StatAggregationType>,
    },
}

impl StatMsg {
    pub fn name(&self) -> &String {
        match self {
            StatMsg::InputStat { name, .. } => name,
            StatMsg::HistoricalStat { name, .. } => name,
            StatMsg::StatWithAggregation { name, .. } => name,
        }
    }

    pub fn value(&self) -> &StatValue {
        match self {
            StatMsg::InputStat { value, .. } => value,
            StatMsg::HistoricalStat { value, .. } => value,
            StatMsg::StatWithAggregation { value, .. } => value,
        }
    }
}

#[cw_serde]
pub enum StatAggregationType {
    Average,
    Cumulative,
}

#[cw_serde]
pub struct StatTableEntry {
    pub addr: Addr,
    pub stats: Vec<StatMsg>,
}

#[cw_serde]
pub struct StatType {
    pub name: String,
    pub value_type: StatValueType,
    pub aggregation_type: Option<StatAggregationType>,
    pub is_beneficial: bool,
    pub tie_breaker_priority: Option<u8>,
}

#[cw_serde]
pub enum StatValueType {
    Bool,
    Decimal,
    Uint,
}

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
