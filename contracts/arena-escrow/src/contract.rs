use crate::{
    execute, migrate, query,
    state::{self, DUE, INITIAL_DUE, IS_LOCKED},
    ContractError,
};
use arena_interface::escrow::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_balance::MemberBalanceUnchecked;

// version info for migration info
pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-escrow";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    instantiate_contract(deps, &info, msg.dues)?;

    Ok(Response::default())
}

pub fn instantiate_contract(
    deps: DepsMut,
    info: &MessageInfo,
    dues: Vec<MemberBalanceUnchecked>,
) -> Result<(), ContractError> {
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    if dues.is_empty() {
        return Err(ContractError::InvalidDue {
            msg: "None due".to_string(),
        });
    }

    IS_LOCKED.save(deps.storage, &false)?;
    for member_balance in dues {
        let member_balance = member_balance.into_checked(deps.as_ref())?;

        if INITIAL_DUE.has(deps.storage, &member_balance.addr) {
            return Err(ContractError::StdError(
                cosmwasm_std::StdError::GenericErr {
                    msg: "Cannot have duplicate addresses in dues".to_string(),
                },
            ));
        }

        INITIAL_DUE.save(deps.storage, &member_balance.addr, &member_balance.balance)?;
        DUE.save(deps.storage, &member_balance.addr, &member_balance.balance)?;
    }

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNative {} => execute::receive_native(deps, info),
        ExecuteMsg::Withdraw {
            cw20_msg,
            cw721_msg,
        } => execute::withdraw(deps, info, cw20_msg, cw721_msg),
        ExecuteMsg::Receive(cw20_receive_msg) => {
            execute::receive_cw20(deps, info, cw20_receive_msg)
        }
        ExecuteMsg::ReceiveNft(cw721_receive_msg) => {
            execute::receive_cw721(deps, info, cw721_receive_msg)
        }
        ExecuteMsg::Distribute {
            distribution,
            layered_fees,
            activation_height,
            group_contract,
        } => execute::distribute(
            deps,
            info,
            distribution,
            layered_fees,
            activation_height,
            group_contract,
        ),
        ExecuteMsg::Lock { value } => execute::lock(deps, info, value),
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { addr } => to_json_binary(&query::balance(deps, addr)?),
        QueryMsg::Due { addr } => to_json_binary(&query::due(deps, addr)?),
        QueryMsg::TotalBalance {} => to_json_binary(&query::total_balance(deps)?),
        QueryMsg::IsLocked {} => to_json_binary(&query::is_locked(deps)),
        QueryMsg::IsFunded { addr } => to_json_binary(&query::is_funded(deps, addr)?),
        QueryMsg::IsFullyFunded {} => to_json_binary(&state::is_fully_funded(deps)),
        QueryMsg::Balances { start_after, limit } => {
            to_json_binary(&query::balances(deps, start_after, limit)?)
        }
        QueryMsg::Dues { start_after, limit } => {
            to_json_binary(&query::dues(deps, start_after, limit)?)
        }
        QueryMsg::InitialDues { start_after, limit } => {
            to_json_binary(&query::initial_dues(deps, start_after, limit)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::DumpState { addr } => to_json_binary(&query::dump_state(deps, addr)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if version.major == 1 && version.minor == 8 {
        migrate::from_v1_8_2_to_v2(deps.branch())?;
    }

    if version.major == 2 && version.minor == 0 {
        migrate::from_v2_to_v2_1(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
