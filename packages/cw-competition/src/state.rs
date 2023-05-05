use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_utils::Expiration;

#[cw_serde]
pub enum CompetitionStatus {
    Pending,
    Active,
    Inactive,
    Jailed,
}

impl Default for CompetitionStatus {
    fn default() -> Self {
        CompetitionStatus::Inactive
    }
}
impl CompetitionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompetitionStatus::Pending => "Pending",
            CompetitionStatus::Jailed => "Jailed",
            CompetitionStatus::Active => "Active",
            CompetitionStatus::Inactive => "Inactive",
        }
    }
}

#[cw_serde]
pub struct Competition<CompetitionExt> {
    pub id: Uint128,
    pub dao: Addr,
    pub escrow: Addr,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub expiration: Expiration,
    pub rules: Vec<String>,
    pub ruleset: Option<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
}

#[cw_serde]
pub struct Config {
    pub key: String,
    pub description: String,
}
