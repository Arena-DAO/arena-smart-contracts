use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Uint128};
use cw_utils::Expiration;

#[cw_serde]
pub enum CompetitionStatus {
    Created,
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
            CompetitionStatus::Created => "Created",
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
pub struct CompetitionResponse<CompetitionExt> {
    pub id: Uint128,
    pub dao: Addr,
    pub escrow: Addr,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub is_expired: bool,
    pub rules: Vec<String>,
    pub ruleset: Option<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
}

impl<CompetitionExt> Competition<CompetitionExt> {
    pub fn to_response(self, block: &BlockInfo) -> CompetitionResponse<CompetitionExt> {
        let is_expired = self.expiration.is_expired(block);

        CompetitionResponse {
            id: self.id,
            dao: self.dao,
            escrow: self.escrow,
            name: self.name,
            description: self.description,
            start_height: self.start_height,
            is_expired,
            rules: self.rules,
            ruleset: self.ruleset,
            status: self.status,
            extension: self.extension,
        }
    }
}

#[cw_serde]
pub struct Config {
    pub key: String,
    pub description: String,
}
