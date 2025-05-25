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
    pub dao_config: TeamDaoConfig,
}

/// Counter to generate unique IDs
pub const TEAM_ENTRY_COUNT: Item<u64> = Item::new("team_entry_count");

/// Stores the applicant status keyed by (team_entry_id, applicant address).
pub const APPLICANTS: Map<(u64, &Addr), ApplicantStatus> = Map::new("applicants");
pub const APPROVED_APPLICANTS: Map<(u64, &Addr), ()> = Map::new("approved_applicants");

/// Stores a map of user teams (user, team)
pub const USER_TEAMS: Map<(&Addr, &Addr), ()> = Map::new("user_teams");

/// Indexes for querying by category_id and status
pub struct TeamEntryIndexes<'a> {
    pub category_status: MultiIndex<'a, (u128, String), TeamEntry, u64>,
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
