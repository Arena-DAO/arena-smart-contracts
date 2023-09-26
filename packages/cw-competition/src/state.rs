use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Uint128};
use cw_balance::MemberShare;
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
    pub admin_dao: Addr,
    pub dao: Addr,
    pub escrow: Option<Addr>,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub expiration: Expiration,
    pub rules: Vec<String>,
    pub rulesets: Vec<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    pub has_generated_proposals: bool,
    pub result: Option<Vec<MemberShare<Addr>>>,
}

/// CompetitionResponse has all of the same fields as Competition
/// is_expired is appended
#[cw_serde]
pub struct CompetitionResponse<CompetitionExt> {
    pub id: Uint128,
    pub dao: Addr,
    pub escrow: Option<Addr>,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub is_expired: bool,
    pub rules: Vec<String>,
    pub rulesets: Vec<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    pub has_generated_proposals: bool,
    pub expiration: Expiration,
    pub result: Option<Vec<MemberShare<Addr>>>,
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
            rulesets: self.rulesets,
            status: self.status,
            extension: self.extension,
            has_generated_proposals: self.has_generated_proposals,
            expiration: self.expiration,
            result: self.result,
        }
    }
}

#[cw_serde]
pub struct Config {
    pub key: String,
    pub description: String,
}
