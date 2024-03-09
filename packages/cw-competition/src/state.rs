use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Timestamp, Uint128};
use cw_utils::Expiration;
use std::fmt;

#[cw_serde]
#[derive(Default)]
pub enum CompetitionStatus {
    Pending,
    Active,
    #[default]
    Inactive,
    Jailed,
}

impl fmt::Display for CompetitionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompetitionStatus::Pending => write!(f, "Pending"),
            CompetitionStatus::Jailed => write!(f, "Jailed"),
            CompetitionStatus::Active => write!(f, "Active"),
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
    pub rulesets: Vec<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
}

/// CompetitionResponse has all of the same fields as Competition
/// is_expired is appended
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
    pub rules: Vec<String>,
    pub rulesets: Vec<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    pub expiration: Expiration,
}

#[cw_serde]
pub struct CompetitionListItemResponse<CompetitionExt> {
    pub id: Uint128,
    pub category_id: Option<Uint128>,
    pub host: Addr,
    pub escrow: Option<Addr>,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub is_expired: bool,
    pub rulesets: Vec<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    pub expiration: Expiration,
}

impl<CompetitionExt> Competition<CompetitionExt> {
    pub fn into_response(
        self,
        rules: Vec<String>,
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
        }
    }

    pub fn into_list_item_response(
        self,
        block: &BlockInfo,
    ) -> CompetitionListItemResponse<CompetitionExt> {
        let is_expired = self.expiration.is_expired(block);

        CompetitionListItemResponse {
            id: self.id,
            category_id: self.category_id,
            host: self.host,
            escrow: self.escrow,
            name: self.name,
            description: self.description,
            start_height: self.start_height,
            is_expired,
            rulesets: self.rulesets,
            status: self.status,
            extension: self.extension,
            expiration: self.expiration,
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
    pub submit_user: Addr,
    pub content: String,
    pub submit_time: Timestamp,
}
