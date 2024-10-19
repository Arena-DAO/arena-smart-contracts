use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_utils::Expiration;

use crate::fees::FeeInformation;

use super::state::{Competition, CompetitionStatus};

pub trait IntoCompetitionExt<CompetitionExt> {
    fn into_competition_ext(self) -> CompetitionExt;
}

/// Used for migration
#[cw_serde]
pub struct CompetitionV2<CompetitionExt> {
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
    pub fees: Option<Vec<FeeInformation<Addr>>>,
    pub banner: Option<String>,
}

impl<T> CompetitionV2<T> {
    pub fn into_competition<E>(self, group_contract: Addr) -> Competition<E>
    where
        T: IntoCompetitionExt<E>,
    {
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
            status: self.status,
            extension: self.extension.into_competition_ext(),
            fees: self.fees,
            banner: self.banner,
            group_contract,
        }
    }
}
