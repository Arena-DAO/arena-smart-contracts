use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_address_like::AddressLike;
use cw_storage_plus::Item;

#[cw_serde]
pub struct VestingConfiguration {
    pub upfront_ratio: Decimal,
    pub vesting_time: u64,
    pub denom: String,
    pub cw_vesting_code_id: u64,
}

#[cw_serde]
pub struct ApplicationInfo<T: AddressLike> {
    pub id: u64,
    pub applicant: T,
    pub title: String,
    pub description: Option<String>,
    pub project_info: Option<String>,
    pub team_info: Option<String>,
    pub status: ApplicationStatus,
}

#[cw_serde]
pub enum ApplicationStatus {
    Pending,
    Accepted,
    Rejected { reason: Option<String> },
}

pub const VESTING_CONFIGURATION: Item<VestingConfiguration> = Item::new("vesting_configuration");
