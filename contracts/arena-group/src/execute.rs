use arena_interface::group::{AddMemberMsg, MemberData, MemberMsg};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, Uint64};
use cw_ownable::assert_owner;

use crate::{
    state::{members, MEMBER_COUNT, TOTAL_POWER},
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
    let mut total_power = TOTAL_POWER.may_load(deps.storage)?.unwrap_or_default();

    if let Some(add_list) = to_add {
        for AddMemberMsg { addr, seed, power } in add_list {
            let addr = deps.api.addr_validate(&addr)?;
            if members().may_load(deps.storage, &addr)?.is_some() {
                return Err(ContractError::DuplicateMembers { member: addr });
            }

            member_count += Uint64::one();
            total_power += power;
            members().save(
                deps.storage,
                &addr,
                &MemberData {
                    seed: seed.unwrap_or(member_count),
                    power,
                },
                env.block.height,
            )?;
        }
    }

    if let Some(update_list) = to_update {
        for MemberMsg { addr, data } in update_list {
            let addr = deps.api.addr_validate(&addr)?;

            if let Some(old_data) = members().may_load(deps.storage, &addr)? {
                // Subtract old power and add new power
                total_power = total_power.checked_sub(old_data.power)?;
                total_power = total_power.checked_add(data.power)?;

                members()
                    .update::<_, StdError>(deps.storage, &addr, env.block.height, |_| Ok(data))?;
            } else {
                return Err(ContractError::NotMember { member: addr });
            }
        }
    }

    if let Some(remove_list) = to_remove {
        for addr_str in remove_list {
            let addr = deps.api.addr_validate(&addr_str)?;
            if let Some(data) = members().may_load(deps.storage, &addr)? {
                total_power = total_power.checked_sub(data.power)?;
                members().remove(deps.storage, &addr, env.block.height)?;
                member_count -= Uint64::one();
            } else {
                return Err(ContractError::NotMember { member: addr });
            }
        }
    }

    MEMBER_COUNT.save(deps.storage, &member_count, env.block.height)?;
    TOTAL_POWER.save(deps.storage, &total_power, env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "update_members")
        .add_attribute("member_count", member_count.to_string())
        .add_attribute("total_power", total_power.to_string()))
}
