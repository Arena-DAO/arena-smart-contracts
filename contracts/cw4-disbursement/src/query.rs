use cosmwasm_std::{Addr, Deps, Env, StdError, StdResult, Uint128};
use cw2::get_contract_version;
use cw4::Member;
use cw4_group::state::{ADMIN, MEMBERS, TOTAL};
use cw_disbursement::DisbursementDataResponse;
use dao_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::{model::DumpStateResponse, state::DISBURSEMENT_DATA};

pub fn disbursement_data(deps: Deps, key: Option<String>) -> StdResult<DisbursementDataResponse> {
    if key.is_none() {
        return Err(StdError::NotFound {
            kind: "Key".to_string(),
        });
    }

    let disbursement_data = DISBURSEMENT_DATA.may_load(deps.storage, key.unwrap())?;

    Ok(DisbursementDataResponse { disbursement_data })
}

pub fn last_updated(deps: Deps) -> StdResult<u64> {
    let last: Vec<u64> = MEMBERS
        .changelog()
        .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .take(1)
        .map(|x| x.map(|y| y.0 .1))
        .collect::<StdResult<_>>()?;

    Ok(*last.first().unwrap_or(&0u64))
}

pub fn info(deps: Deps) -> StdResult<InfoResponse> {
    let info = get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

pub fn dump_state(deps: Deps) -> StdResult<DumpStateResponse> {
    let total_weight = TOTAL.may_load(deps.storage)?.unwrap_or_default();
    let members = MEMBERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .map(|item| {
            item.map(|x| Member {
                addr: x.0.to_string(),
                weight: x.1,
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(DumpStateResponse {
        members,
        total_weight,
    })
}

pub fn total_weight_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let power = match height {
        Some(val) => TOTAL.may_load_at_height(deps.storage, val),
        None => TOTAL.may_load(deps.storage),
    }?
    .unwrap_or_default();
    Ok(TotalPowerAtHeightResponse {
        power: Uint128::from(power),
        height: height.unwrap_or(env.block.height),
    })
}

pub fn voting_power_at_height(
    deps: Deps,
    env: Env,
    addr: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let weight = match height {
        Some(h) => MEMBERS.may_load_at_height(deps.storage, &addr, h),
        None => MEMBERS.may_load(deps.storage, &addr),
    }?
    .unwrap_or_default();
    Ok(VotingPowerAtHeightResponse {
        power: Uint128::from(weight),
        height: height.unwrap_or(env.block.height),
    })
}

pub fn dao(deps: Deps) -> StdResult<Addr> {
    Ok(ADMIN.get(deps)?.unwrap())
}
