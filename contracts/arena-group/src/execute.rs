use arena_interface::group::{AddMemberMsg, MemberMsg};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, Uint64};
use cw_ownable::assert_owner;

use crate::{
    state::{members, MEMBER_COUNT},
    ContractError,
};

pub fn update_members(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_add: Option<Vec<AddMemberMsg>>,
    to_update: Option<Vec<MemberMsg<String>>>,
    to_remove: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    if info.sender != env.contract.address {
        assert_owner(deps.storage, &info.sender)?;
    }

    let mut member_count = MEMBER_COUNT.may_load(deps.storage)?.unwrap_or_default();

    if let Some(add_list) = to_add {
        for AddMemberMsg { addr, seed } in add_list {
            let addr = deps.api.addr_validate(&addr)?;
            if members().has(deps.storage, &addr) {
                return Err(ContractError::DuplicateMembers { member: addr });
            }

            member_count += Uint64::one();
            members().save(deps.storage, &addr, &seed.unwrap_or(member_count))?;
        }
    }

    if let Some(update_list) = to_update {
        for MemberMsg { addr, seed } in update_list {
            let addr = deps.api.addr_validate(&addr)?;

            if members().has(deps.storage, &addr) {
                members().update::<_, StdError>(deps.storage, &addr, |_| Ok(seed))?;
            } else {
                return Err(ContractError::NotMember { member: addr });
            }
        }
    }

    if let Some(remove_list) = to_remove {
        for addr_str in remove_list {
            let addr = deps.api.addr_validate(&addr_str)?;
            if members().has(deps.storage, &addr) {
                members().remove(deps.storage, &addr)?;
                member_count -= Uint64::one();
            } else {
                return Err(ContractError::NotMember { member: addr });
            }
        }
    }

    MEMBER_COUNT.save(deps.storage, &member_count)?;

    Ok(Response::new()
        .add_attribute("action", "update_members")
        .add_attribute("member_count", member_count.to_string()))
}
