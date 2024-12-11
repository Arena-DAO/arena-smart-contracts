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
    pub applicant: Addr,
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
pub const APPLICATIONS_COUNT: Item<Uint128> = Item::new("applications_count");

pub struct ApplicationIndexes<'a> {
    pub status: MultiIndex<'a, String, ApplicationInfo, u128>,
    pub applicant: MultiIndex<'a, Addr, ApplicationInfo, u128>,
}

impl IndexList<ApplicationInfo> for ApplicationIndexes<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<ApplicationInfo>> + '_> {
        let v: Vec<&dyn Index<ApplicationInfo>> = vec![&self.status, &self.applicant];
        Box::new(v.into_iter())
    }
}

pub fn applications<'a>() -> IndexedMap<'a, u128, ApplicationInfo, ApplicationIndexes<'a>> {
    let indexes = ApplicationIndexes {
        status: MultiIndex::new(
            |_pk, d: &ApplicationInfo| d.status.to_string(),
            "applications",
            "applications__status",
        ),
        applicant: MultiIndex::new(
            |_pk, d: &ApplicationInfo| d.applicant.clone(),
            "applications",
            "applications__applicant",
        ),
    };
    IndexedMap::new("applications", indexes)
}
