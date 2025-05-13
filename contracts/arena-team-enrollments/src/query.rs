use crate::msg::{ApplicantResponse, TeamEntryResponse};
use crate::state::{team_entries, EntryStatus, APPLICANTS, USER_TEAMS};
use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

/// Query a single team entry by its ID
pub fn get_entry(deps: Deps, entry_id: u64) -> StdResult<TeamEntryResponse> {
    let entry = team_entries().load(deps.storage, entry_id)?;
    Ok(TeamEntryResponse {
        entry_id,
        creator: entry.creator,
        title: entry.title,
        description: entry.description,
        category_id: entry.category_id,
        status: entry.status,
        created_at: entry.created_at.seconds(),
    })
}

/// List team entries with optional category_id and status filters
pub fn list_entries(
    deps: Deps,
    category_id: Option<Uint128>,
    status: Option<EntryStatus>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<TeamEntryResponse>> {
    let entries = team_entries();
    let lim = limit.unwrap_or(10).min(50) as usize;
    let start = start_after.map(|id| id + 1);

    let filtered: Box<dyn Iterator<Item = StdResult<(u64, _)>>> = match (category_id, status) {
        (Some(cat), Some(st)) => {
            let idx = entries.idx.category_status;
            Box::new(idx.prefix((cat.u128(), st.to_string())).range(
                deps.storage,
                None,
                None,
                Order::Ascending,
            ))
        }
        _ => {
            // fallback to full scan
            Box::new(entries.range(
                deps.storage,
                start.map(Bound::exclusive),
                None,
                Order::Ascending,
            ))
        }
    };

    filtered
        .take(lim)
        .map(|res| {
            let (entry_id, entry) = res?;
            Ok(TeamEntryResponse {
                entry_id,
                creator: entry.creator,
                title: entry.title,
                description: entry.description,
                category_id: entry.category_id,
                status: entry.status,
                created_at: entry.created_at.seconds(),
            })
        })
        .collect()
}

/// Get applicant's status for a specific entry
pub fn get_applicant(deps: Deps, entry_id: u64, applicant: String) -> StdResult<ApplicantResponse> {
    let applicant = deps.api.addr_validate(&applicant)?;
    let status = APPLICANTS.load(deps.storage, (entry_id, &applicant))?;
    Ok(ApplicantResponse { applicant, status })
}

/// List all applicants for a specific entry, with optional pagination.
pub fn list_applicants(
    deps: Deps,
    entry_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<ApplicantResponse>> {
    let lim = limit.unwrap_or(10).min(50);
    let start = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let start_bound = start.as_ref().map(Bound::exclusive);

    APPLICANTS
        .prefix(entry_id)
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(lim as usize)
        .map(|res| {
            let (applicant, status) = res?;
            Ok(ApplicantResponse { applicant, status })
        })
        .collect()
}

pub fn list_user_teams(
    deps: Deps,
    user: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Addr>> {
    let user = deps.api.addr_validate(&user)?;
    let start = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let start_bound = start.as_ref().map(Bound::exclusive);
    let lim = limit.unwrap_or(10).min(50);

    USER_TEAMS
        .prefix(&user)
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(lim as usize)
        .map(|res| {
            let (addr, _) = res?;
            Ok(addr)
        })
        .collect()
}
