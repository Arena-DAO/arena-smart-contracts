use crate::msg::{ApplicantResponse, CategoryStatusMsg, TeamEntryResponse};
use crate::state::{applicants, team_entries, ApplicantStatus, APPLICANTS_COUNT, USER_TEAMS};
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::Bound;

/// Query a single team entry by its ID
pub fn get_entry(deps: Deps, entry_id: u64) -> StdResult<TeamEntryResponse> {
    let entry = team_entries().load(deps.storage, entry_id)?;
    let (pending_applicants_count, approved_applicants_count, rejected_applicants_count) =
        get_applicant_counts(deps, entry_id)?;

    Ok(TeamEntryResponse {
        entry_id,
        team_entry: entry,
        pending_applicants_count,
        approved_applicants_count,
        rejected_applicants_count,
    })
}

fn get_applicant_counts(deps: Deps, entry_id: u64) -> StdResult<(u64, u64, u64)> {
    let pending_applicants_count = APPLICANTS_COUNT
        .may_load(deps.storage, (entry_id, ApplicantStatus::Default.as_str()))?
        .unwrap_or_default();
    let approved_applicants_count = APPLICANTS_COUNT
        .may_load(deps.storage, (entry_id, ApplicantStatus::Approved.as_str()))?
        .unwrap_or_default();
    let rejected_applicants_count = APPLICANTS_COUNT
        .may_load(
            deps.storage,
            (
                entry_id,
                ApplicantStatus::Rejected {
                    reason: String::default(),
                }
                .as_str(),
            ),
        )?
        .unwrap_or_default();

    Ok((
        pending_applicants_count,
        approved_applicants_count,
        rejected_applicants_count,
    ))
}

/// List team entries with optional category_id and status filters
pub fn list_entries(
    deps: Deps,
    category_status: Option<CategoryStatusMsg>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<TeamEntryResponse>> {
    let entries = team_entries();
    let lim = limit.unwrap_or(10).min(50) as usize;
    let start = start_after.map(|id| id + 1);

    let filtered: Box<dyn Iterator<Item = StdResult<(u64, _)>>> = match category_status {
        Some(CategoryStatusMsg {
            category_id,
            status,
        }) => {
            let idx = entries.idx.category_status;
            Box::new(
                idx.prefix((category_id.unwrap_or_default().u128(), status.as_str()))
                    .range(deps.storage, None, None, Order::Ascending),
            )
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
            let (pending_applicants_count, approved_applicants_count, rejected_applicants_count) =
                get_applicant_counts(deps, entry_id)?;

            Ok(TeamEntryResponse {
                entry_id,
                team_entry: entry,
                pending_applicants_count,
                approved_applicants_count,
                rejected_applicants_count,
            })
        })
        .collect()
}

/// Get applicant's status for a specific entry
pub fn get_applicant(deps: Deps, entry_id: u64, applicant: String) -> StdResult<ApplicantResponse> {
    let applicant = deps.api.addr_validate(&applicant)?;
    let status = applicants().load(deps.storage, (entry_id, &applicant))?;
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

    applicants()
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
