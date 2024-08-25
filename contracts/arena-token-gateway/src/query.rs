use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::{
    msg::ApplicationResponse,
    state::{applications, ApplicationStatus},
};

pub fn application(deps: Deps, applicant: String) -> StdResult<ApplicationResponse> {
    let applicant_addr = deps.api.addr_validate(&applicant)?;
    let application = applications().load(deps.storage, &applicant_addr)?;
    Ok(ApplicationResponse {
        applicant,
        application,
    })
}

pub fn list_applications(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    status: Option<ApplicationStatus>,
) -> StdResult<Vec<ApplicationResponse>> {
    let limit = limit.unwrap_or(30) as usize;
    let validated_start_after = start_after
        .map(|s| deps.api.addr_validate(&s))
        .transpose()?;
    let start = validated_start_after.as_ref().map(Bound::exclusive);

    let applications: StdResult<Vec<ApplicationResponse>> = match status {
        Some(status) => applications()
            .idx
            .status
            .prefix(status.to_string())
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (applicant, application) = item?;
                Ok(ApplicationResponse {
                    applicant: applicant.to_string(),
                    application,
                })
            })
            .collect(),
        None => applications()
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (applicant, application) = item?;
                Ok(ApplicationResponse {
                    applicant: applicant.to_string(),
                    application,
                })
            })
            .collect(),
    };

    applications
}
