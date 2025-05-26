use std::fmt;

use arena_interface::competition::types::DaoConfig;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, StdResult, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

#[cw_serde]
pub struct TeamDaoConfig {
    #[serde(flatten)]
    pub dao_config: DaoConfig,
    pub cw4_group_code_id: u64,
}

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

impl EntryStatus {
    pub fn validate_transition(&self, new_status: &EntryStatus) -> StdResult<()> {
        use EntryStatus::*;

        match (self, new_status) {
            // No-op transition disallowed
            (cur, new) if cur == new => {
                Err(StdError::generic_err("No-op transitions are not allowed"))
            }

            // Open can go to anything
            (Open, _) => Ok(()),

            // Closed can reopen
            (Closed, Open) => Ok(()),

            // Created and Aborted are terminal
            (Created, _) => Err(StdError::generic_err("Created is a final state")),
            (Aborted, _) => Err(StdError::generic_err("Aborted is a final state")),

            // All other transitions are invalid
            _ => Err(StdError::generic_err("Invalid entry status transition")),
        }
    }
}

impl fmt::Display for EntryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = self.as_str();
        write!(f, "{}", status_str)
    }
}

impl EntryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntryStatus::Open => "open",
            EntryStatus::Created => "created",
            EntryStatus::Closed => "closed",
            EntryStatus::Aborted => "aborted",
        }
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

impl ApplicantStatus {
    pub fn validate_transition(&self, new_status: &ApplicantStatus) -> StdResult<()> {
        match (self, new_status) {
            // Default → Approved or Rejected (with reason)
            (ApplicantStatus::Default, ApplicantStatus::Approved) => Ok(()),
            (ApplicantStatus::Default, ApplicantStatus::Rejected { reason })
                if !reason.trim().is_empty() =>
            {
                Ok(())
            }

            // Approved ↔ Rejected (with reason)
            (ApplicantStatus::Approved, ApplicantStatus::Rejected { reason })
                if !reason.trim().is_empty() =>
            {
                Ok(())
            }
            (ApplicantStatus::Rejected { .. }, ApplicantStatus::Approved) => Ok(()),

            // Disallow no-op
            (cur, new) if cur == new => {
                Err(StdError::generic_err("No-op transitions are not allowed"))
            }

            // All other transitions are invalid
            _ => Err(StdError::generic_err("Invalid applicant status transition")),
        }
    }
}

impl fmt::Display for ApplicantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.as_str();
        write!(f, "{}", s)
    }
}

impl ApplicantStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicantStatus::Default => "default",
            ApplicantStatus::Approved => "approved",
            ApplicantStatus::Rejected { .. } => "rejected",
        }
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
    pub dao_config: TeamDaoConfig,
}

/// Counter to generate unique IDs
pub const TEAM_ENTRY_COUNT: Item<u64> = Item::new("team_entry_count");

/// Stores the applicant status keyed by (team_entry_id, applicant address) -> applicant status.
pub struct ApplicantsIndexes<'a> {
    pub entry_status: MultiIndex<'a, (u64, &'a str), ApplicantStatus, (u64, &'a Addr)>,
}

impl IndexList<ApplicantStatus> for ApplicantsIndexes<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<ApplicantStatus>> + '_> {
        let v: Vec<&dyn Index<ApplicantStatus>> = vec![&self.entry_status];
        Box::new(v.into_iter())
    }
}

/// IndexedMap for storing team entries
pub fn applicants<'a>() -> IndexedMap<(u64, &'a Addr), ApplicantStatus, ApplicantsIndexes<'a>> {
    let entry_status_idx = MultiIndex::new(
        |pk, d: &ApplicantStatus| {
            // The composite key (u64, &Addr) is encoded with length prefixes
            // Format: [u64_len_prefix][u64_bytes][addr_len_prefix][addr_bytes]

            if pk.len() < 10 {
                // Need at least 2 bytes prefix + 8 bytes u64
                return (0u64, d.as_str());
            }
            // Try to decode assuming the format includes a 2-byte length prefix for u64
            // Skip first 2 bytes (length prefix) and read the next 8 bytes as u64
            let entry_id_bytes: [u8; 8] = pk[2..10].try_into().unwrap_or([0; 8]);
            let entry_id = u64::from_be_bytes(entry_id_bytes);
            (entry_id, d.as_str())
        },
        "applicants",
        "applicants__entry_status",
    );

    IndexedMap::new(
        "applicants",
        ApplicantsIndexes {
            entry_status: entry_status_idx,
        },
    )
}

/// Count of applicants on an entry with status (team_entry_id, status)
pub const APPLICANTS_COUNT: Map<(u64, &str), u64> = Map::new("applicants_count");

/// Stores a map of user teams (user, team)
pub const USER_TEAMS: Map<(&Addr, &Addr), ()> = Map::new("user_teams");

/// Indexes for querying by category_id and status
pub struct TeamEntryIndexes<'a> {
    pub category_status: MultiIndex<'a, (u128, &'a str), TeamEntry, u64>,
}

impl IndexList<TeamEntry> for TeamEntryIndexes<'_> {
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
            let status = d.status.as_str();
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
