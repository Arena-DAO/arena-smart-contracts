use crate::{
    execute, migrate,
    msg::{InstantiateMsg, MigrateMsg},
    query,
    state::{self, DUE, INITIAL_DUE, IS_LOCKED, SHOULD_ACTIVATE_ON_FUNDED},
    ContractError,
};
use arena_interface::escrow::{ExecuteMsg, QueryMsg};
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
    instantiate_contract(deps, info, msg.dues, msg.should_activate_on_funded)?;
    Ok(Response::default())
}

pub fn instantiate_contract(
    deps: DepsMut,
    info: MessageInfo,
    due: Vec<MemberBalanceUnchecked>,
    should_activate_on_funded: Option<bool>,
) -> Result<(), ContractError> {
    if due.is_empty() {
        return Err(ContractError::InvalidDue {
            msg: "None due".to_string(),
        });
    }

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;
    IS_LOCKED.save(deps.storage, &false)?;
    if let Some(should_activate_on_funded) = should_activate_on_funded {
        SHOULD_ACTIVATE_ON_FUNDED.save(deps.storage, &should_activate_on_funded)?;
    }
    for member_balance in due {
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
        ExecuteMsg::SetDistribution { distribution } => {
            execute::set_distribution(deps, info, distribution)
        }
        ExecuteMsg::Receive(cw20_receive_msg) => {
            execute::receive_cw20(deps, info, cw20_receive_msg)
        }
        ExecuteMsg::ReceiveNft(cw721_receive_msg) => {
            execute::receive_cw721(deps, info, cw721_receive_msg)
        }
        ExecuteMsg::Activate {} => execute::activate(deps, env, info),
        ExecuteMsg::Distribute {
            distribution,
            layered_fees,
        } => execute::distribute(deps, info, distribution, layered_fees),
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
        QueryMsg::Distribution { addr } => to_json_binary(&query::distribution(deps, addr)?),
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
        QueryMsg::ShouldActivateOnFunded {} => {
            to_json_binary(&query::should_activate_on_funded(deps)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if version.major == 1 && version.minor == 3 {
        migrate::from_v1_3_to_v_1_4(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
