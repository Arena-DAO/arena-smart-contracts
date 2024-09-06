use cosmwasm_std::{Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::{
    msg::{ApplicationResponse, ApplicationsFilter},
    state::applications,
};

pub fn application(deps: Deps, application_id: Uint128) -> StdResult<ApplicationResponse> {
    let application = applications().load(deps.storage, application_id.u128())?;
    Ok(ApplicationResponse {
        application_id,
        application,
    })
}

pub fn list_applications(
    deps: Deps,
    start_after: Option<Uint128>,
    limit: Option<u32>,
    filter: Option<ApplicationsFilter>,
) -> StdResult<Vec<ApplicationResponse>> {
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(|x| x.u128()).map(Bound::exclusive);

    let applications: StdResult<Vec<ApplicationResponse>> = match filter {
        Some(filter) => match filter {
            ApplicationsFilter::Status(status) => applications()
                .idx
                .status
                .prefix(status.to_string())
                .range(deps.storage, start, None, Order::Ascending)
                .take(limit)
                .map(|item| {
                    let (application_id, application) = item?;
                    Ok(ApplicationResponse {
                        application_id: Uint128::new(application_id),
                        application,
                    })
                })
                .collect(),
            ApplicationsFilter::Applicant(applicant) => {
                let applicant = deps.api.addr_validate(&applicant)?;

                applications()
                    .idx
                    .applicant
                    .prefix(applicant)
                    .range(deps.storage, start, None, Order::Ascending)
                    .take(limit)
                    .map(|item| {
                        let (application_id, application) = item?;
                        Ok(ApplicationResponse {
                            application_id: Uint128::new(application_id),
                            application,
                        })
                    })
                    .collect()
            }
        },
        None => applications()
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (application_id, application) = item?;
                Ok(ApplicationResponse {
                    application_id: Uint128::new(application_id),
                    application,
                })
            })
            .collect(),
    };

    applications
}
