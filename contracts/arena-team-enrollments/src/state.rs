use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

/// Enum representing the status of a team entry
#[derive(Default)]
#[cw_serde]
pub enum EntryStatus {
    #[default]
    Open,
    Created,
    Closed,
    Aborted,
}

impl fmt::Display for EntryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = match self {
            EntryStatus::Open => "open",
            EntryStatus::Created => "created",
            EntryStatus::Closed => "closed",
            EntryStatus::Aborted => "aborted",
        };
        write!(f, "{}", status_str)
    }
}

#[derive(Default)]
#[cw_serde]
pub enum ApplicantStatus {
    #[default]
    Default,
    Approved,
    Rejected {
        reason: String,
    },
}

impl fmt::Display for ApplicantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ApplicantStatus::Default => "default",
            ApplicantStatus::Approved => "approved",
            ApplicantStatus::Rejected { .. } => "rejected",
        };
        write!(f, "{}", s)
    }
}

/// Main structure for a team entry
#[cw_serde]
pub struct TeamEntry {
    pub creator: Addr,
    pub title: String,
    pub description: String,
    pub category_id: Option<Uint128>,
    pub status: EntryStatus,
    pub created_at: Timestamp,
}

/// Counter to generate unique IDs
pub const TEAM_ENTRY_COUNT: Item<u64> = Item::new("team_entry_count");

/// Indexes for querying by category_id and status
pub struct TeamEntryIndexes<'a> {
    pub category_status: MultiIndex<'a, (u128, String), TeamEntry, u64>,
}

impl<'a> IndexList<TeamEntry> for TeamEntryIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TeamEntry>> + '_> {
        let v: Vec<&dyn Index<TeamEntry>> = vec![&self.category_status];
        Box::new(v.into_iter())
    }
}

/// IndexedMap for storing team entries
pub fn team_entries<'a>() -> IndexedMap<u64, TeamEntry, TeamEntryIndexes<'a>> {
    let category_status_idx = MultiIndex::new(
        |_pk, d: &TeamEntry| {
            let category = d.category_id.map(|x| x.u128()).unwrap_or_default();
            let status = d.status.to_string();
            (category, status)
        },
        "team_entries",
        "team_entries__category_status",
    );

    IndexedMap::new(
        "team_entries",
        TeamEntryIndexes {
            category_status: category_status_idx,
        },
    )
}

pub struct ApplicantIndexes<'a> {
    pub status: MultiIndex<'a, String, ApplicantStatus, (u64, &'a Addr)>,
}

impl<'a> IndexList<ApplicantStatus> for ApplicantIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<ApplicantStatus>> + '_> {
        let v: Vec<&dyn Index<ApplicantStatus>> = vec![&self.status];
        Box::new(v.into_iter())
    }
}

pub fn applicants<'a>() -> IndexedMap<(u64, Addr), ApplicantStatus, ApplicantIndexes<'a>> {
    let status_idx = MultiIndex::new(
        |_pk, d: &ApplicantStatus| d.to_string(),
        "applicants",
        "applicants__status",
    );

    IndexedMap::new("applicants", ApplicantIndexes { status: status_idx })
}
