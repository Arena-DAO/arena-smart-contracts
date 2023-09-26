use cosmwasm_std::{Addr, Deps, StdResult};
use cw4::{Member, MemberListResponse};

pub fn get_all_members(deps: Deps, cw4_group_addr: &Addr) -> StdResult<Vec<Member>> {
    let mut all_members: Vec<Member> = vec![];
    let mut start_after = None;
    const LIMIT: u32 = 50;

    loop {
        let response: MemberListResponse = deps.querier.query_wasm_smart(
            cw4_group_addr,
            &cw4::Cw4QueryMsg::ListMembers {
                start_after: start_after.clone(),
                limit: Some(LIMIT),
            },
        )?;

        if response.members.is_empty() {
            break;
        }

        start_after = Some(response.members.last().unwrap().addr.clone());
        all_members.extend(response.members);
    }

    Ok(all_members)
}
