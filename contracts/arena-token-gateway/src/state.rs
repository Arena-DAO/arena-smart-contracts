use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

#[cw_serde]
pub struct VestingConfiguration {
    pub upfront_ratio: Decimal,
    pub vesting_time: u64,
    pub denom: String,
}

#[cw_serde]
pub struct ApplicationInfo {
    pub title: String,
    pub description: String,
    pub requested_amount: Uint128,
    pub project_links: Vec<ProjectLink>,
    pub status: ApplicationStatus,
}

#[cw_serde]
pub struct ProjectLink {
    pub title: String,
    pub url: String,
}

#[cw_serde]
pub enum ApplicationStatus {
    Pending {},
    Accepted {},
    Rejected { reason: Option<String> },
}

pub const VESTING_CONFIGURATION: Item<VestingConfiguration> = Item::new("vesting_configuration");

pub struct ApplicationIndexes<'a> {
    pub status: MultiIndex<'a, String, ApplicationInfo, &'a Addr>,
}

impl<'a> IndexList<ApplicationInfo> for ApplicationIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<ApplicationInfo>> + '_> {
        let v: Vec<&dyn Index<ApplicationInfo>> = vec![&self.status];
        Box::new(v.into_iter())
    }
}

pub fn applications<'a>() -> IndexedMap<'a, &'a Addr, ApplicationInfo, ApplicationIndexes<'a>> {
    let indexes = ApplicationIndexes {
        status: MultiIndex::new(
            |_pk, d: &ApplicationInfo| d.status.to_string(),
            "applications",
            "applications__status",
        ),
    };
    IndexedMap::new("applications", indexes)
}
