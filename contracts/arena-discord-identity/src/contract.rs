use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, WasmMsg,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_ownable::assert_owner;

use crate::{
    msg::{DiscordProfile, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{discord_identity, FAUCET_AMOUNT},
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    let ownership = cw_ownable::initialize_owner(deps.storage, deps.api, Some(owner.as_str()))?;

    Ok(Response::new()
        .add_attributes(ownership.into_attributes())
        .add_message(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::SetFaucetAmount {
                amount: msg.faucet_amount,
            })?,
            funds: vec![],
        }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    if info.sender != env.contract.address {
        assert_owner(deps.storage, &info.sender)?;
    }

    match msg {
        ExecuteMsg::SetProfile {
            addr,
            discord_id,
            username,
            avatar_hash,
            connections,
        } => {
            let discord_identity = discord_identity();
            let user = deps.api.addr_validate(&addr)?;
            let mut msgs = vec![];
            if !discord_identity.has(deps.storage, &user)
                && discord_identity
                    .idx
                    .discord_id
                    .prefix(discord_id.u64())
                    .range(deps.storage, None, None, Order::Descending)
                    .collect::<StdResult<Vec<_>>>()?
                    .is_empty()
            {
                let faucet_amount = FAUCET_AMOUNT.load(deps.storage)?;

                if deps
                    .querier
                    .query_balance(&user, &faucet_amount.denom)?
                    .amount
                    .is_zero()
                {
                    let amount = vec![faucet_amount];
                    msgs.push(BankMsg::Send {
                        to_address: user.to_string(),
                        amount,
                    })
                }
            }

            discord_identity.save(
                deps.storage,
                &user,
                &DiscordProfile {
                    user_id: discord_id,
                    username,
                    avatar_hash,
                    connections,
                },
            )?;

            Ok(Response::new()
                .add_messages(msgs)
                .add_attribute("action", "set_profile")
                .add_attribute("address", user)
                .add_attribute("discord_id", discord_id.to_string()))
        }
        ExecuteMsg::SetFaucetAmount { amount } => {
            FAUCET_AMOUNT.save(deps.storage, &amount)?;

            Ok(Response::new()
                .add_attribute("action", "set_faucet_amount")
                .add_attribute("amount", amount.to_string()))
        }
        ExecuteMsg::Withdraw {} => {
            let funds = deps
                .querier
                .query_all_balances(env.contract.address.to_string())?;

            Ok(Response::new()
                .add_attribute("action", "withdraw")
                .add_message(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: funds,
                }))
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::UserId { addr } => {
            let addr = deps.api.addr_validate(&addr)?;

            to_json_binary(&discord_identity().may_load(deps.storage, &addr)?)
        }
        QueryMsg::ConnectedWallets { discord_id } => to_json_binary(
            &discord_identity()
                .idx
                .discord_id
                .prefix(discord_id.u64())
                .keys(deps.storage, None, None, Order::Descending)
                .collect::<StdResult<Vec<_>>>()?,
        ),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let _version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
