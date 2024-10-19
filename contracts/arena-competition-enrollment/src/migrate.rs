use arena_interface::group;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, CosmosMsg, DepsMut, Env, Order, StdError, StdResult,
    WasmMsg,
};
use sha2::{Digest, Sha256};

use crate::state::{enrollment_entries, enrollment_entriesv2, EnrollmentEntry, ENROLLMENT_MEMBERS};

pub fn from_v2_to_v2_1(deps: DepsMut, env: &Env, group_id: u64) -> StdResult<Vec<CosmosMsg>> {
    deps.storage.remove(b"enrollment_members_count");

    let mut msgs = vec![];
    for (enrollment_id, enrollment) in enrollment_entriesv2()
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        let salt: [u8; 32] = Sha256::digest(enrollment_id.to_string().as_bytes()).into();
        let canonical_creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
        let code_info = deps.querier.query_wasm_code_info(group_id)?;
        let canonical_addr = instantiate2_address(&code_info.checksum, &canonical_creator, &salt)
            .map_err(|x| StdError::generic_err(x.to_string()))?;

        let members = Some(
            ENROLLMENT_MEMBERS
                .prefix(enrollment_id)
                .range(deps.storage, None, None, Order::Ascending)
                .map(|x| {
                    x.map(|y| group::AddMemberMsg {
                        addr: y.0.to_string(),
                        seed: None,
                    })
                })
                .collect::<StdResult<Vec<_>>>()?,
        );

        msgs.push(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
            admin: Some(env.contract.address.to_string()),
            code_id: group_id,
            label: "Arena Group".to_string(),
            msg: to_json_binary(&group::InstantiateMsg { members })?,
            funds: vec![],
            salt: salt.into(),
        }));

        let group_contract = deps.api.addr_humanize(&canonical_addr)?;

        let new_enrollment = EnrollmentEntry {
            min_members: enrollment.min_members,
            max_members: enrollment.max_members,
            entry_fee: enrollment.entry_fee,
            expiration: enrollment.expiration,
            has_triggered_expiration: enrollment.has_triggered_expiration,
            competition_info: enrollment.competition_info,
            competition_type: enrollment.competition_type,
            host: enrollment.host,
            category_id: enrollment.category_id,
            competition_module: enrollment.competition_module,
            group_contract,
        };

        enrollment_entries().replace(
            deps.storage,
            enrollment_id,
            Some(&new_enrollment),
            Some(&new_enrollment),
        )?;
    }

    ENROLLMENT_MEMBERS.clear(deps.storage);

    Ok(msgs)
}
