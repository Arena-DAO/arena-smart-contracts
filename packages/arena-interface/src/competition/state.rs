use crate::fees::FeeInformation;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Timestamp, Uint128};
use cw_utils::Expiration;
use std::fmt;

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
    pub group_contract: Addr,
}

#[cw_serde]
pub struct TempCompetition<CompetitionInstantiateExt> {
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
    /// Additional layered fees
    pub fees: Option<Vec<FeeInformation<Addr>>>,
    /// A banner-image link for the competition
    pub banner: Option<String>,
    pub extension: CompetitionInstantiateExt,
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
    pub group_contract: Addr,
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
            group_contract: self.group_contract,
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
