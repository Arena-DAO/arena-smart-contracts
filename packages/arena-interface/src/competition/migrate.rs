use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_utils::Expiration;

use crate::fees::FeeInformation;

use super::state::{Competition, CompetitionStatus};

/// Used for migration
#[cw_serde]
pub enum CompetitionStatusV182 {
    Pending,
    Active,
    Inactive,
    Jailed,
}

impl fmt::Display for CompetitionStatusV182 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompetitionStatusV182::Pending => write!(f, "Pending"),
            CompetitionStatusV182::Jailed => write!(f, "Jailed"),
            CompetitionStatusV182::Active => write!(f, "Active"),
            CompetitionStatusV182::Inactive => write!(f, "Inactive"),
        }
    }
}

/// Used for migration
#[cw_serde]
pub struct CompetitionV182<CompetitionExt> {
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
    pub status: CompetitionStatusV182,
    pub extension: CompetitionExt,
    pub fees: Option<Vec<FeeInformation<Addr>>>,
    pub banner: Option<String>,
}

impl<CompetitionExt: Clone> CompetitionV182<CompetitionExt> {
    pub fn into_competition(self, activation_height: u64) -> Competition<CompetitionExt> {
        Competition {
            id: self.id,
            category_id: self.category_id,
            admin_dao: self.admin_dao,
            host: self.host,
            escrow: self.escrow,
            name: self.name,
            description: self.description,
            start_height: self.start_height,
            expiration: self.expiration,
            rulesets: self.rulesets,
            status: match self.status {
                CompetitionStatusV182::Pending => CompetitionStatus::Pending,
                CompetitionStatusV182::Active => CompetitionStatus::Active { activation_height },
                CompetitionStatusV182::Inactive => CompetitionStatus::Inactive,
                CompetitionStatusV182::Jailed => CompetitionStatus::Jailed { activation_height },
            },
            extension: self.extension,
            fees: self.fees,
            banner: self.banner,
        }
    }
}
