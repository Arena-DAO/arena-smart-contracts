use cosmwasm_std::{
    to_json_binary, Addr, Attribute, Binary, CosmosMsg, DepsMut, Empty, MessageInfo, Response,
    StdResult,
};
use cw20::{Cw20CoinVerified, Cw20ReceiveMsg};
use cw721::Cw721ReceiveMsg;
use cw_balance::{BalanceVerified, Cw721CollectionVerified, MemberShare};
use cw_ownable::{assert_owner, get_ownership};

use crate::{
    query::is_locked,
    state::{
        is_fully_funded, BALANCE, DUE, INITIAL_DUE, IS_LOCKED, PRESET_DISTRIBUTION, TOTAL_BALANCE,
    },
    ContractError,
};

fn inner_withdraw(
    deps: DepsMut,
    addrs: Vec<Addr>,
    cw20_msg: Option<Binary>,
    cw721_msg: Option<Binary>,
    is_processing: bool,
) -> Result<Response, ContractError> {
    // Initialize total_balance based on processing status
    let mut total_balance = if is_processing {
        BalanceVerified::new()
    } else {
        TOTAL_BALANCE.load(deps.storage)?
    };

    let mut msgs = vec![];
    let mut attrs = vec![];

    for addr in addrs {
        // Load and process balance for each address
        if let Some(balance) = BALANCE.may_load(deps.storage, &addr)? {
            if balance.is_empty() {
                continue;
            }

            // Prepare messages for balance transmission
            msgs.append(&mut balance.transmit_all(
                deps.as_ref(),
                &addr,
                cw20_msg.clone(),
                cw721_msg.clone(),
            )?);

            // Record processed address
            attrs.push(Attribute {
                key: "addr".to_string(),
                value: addr.to_string(),
            });

            // Update total balance and related storage entries
            BALANCE.remove(deps.storage, &addr);
            if !is_processing {
                total_balance = total_balance.checked_sub(&balance)?;

                let initial_due = &INITIAL_DUE.load(deps.storage, &addr)?;
                DUE.save(deps.storage, &addr, initial_due)?;
            }
        }
    }

    // Update or remove total balance based on processing status
    if is_processing {
        TOTAL_BALANCE.remove(deps.storage);
    } else {
        TOTAL_BALANCE.save(deps.storage, &total_balance)?;
    }

    Ok(Response::new()
        .add_attribute("action", "withdraw")
        .add_attributes(attrs)
        .add_messages(msgs))
}

// This function handles refunds for the sender
pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Option<Binary>,
    cw721_msg: Option<Binary>,
) -> Result<Response, ContractError> {
    if is_locked(deps.as_ref()) {
        return Err(ContractError::Locked {});
    }

    inner_withdraw(deps, vec![info.sender], cw20_msg, cw721_msg, false)
}

pub fn set_distribution(
    deps: DepsMut,
    info: MessageInfo,
    distribution: Vec<MemberShare<String>>,
) -> Result<Response, ContractError> {
    // Convert String keys to Addr
    let validated_distribution = distribution
        .into_iter()
        .map(|x| x.to_validated(deps.as_ref()))
        .collect::<StdResult<_>>()?;

    // Save distribution in the state
    PRESET_DISTRIBUTION.save(deps.storage, &info.sender, &validated_distribution)?;

    Ok(Response::new()
        .add_attribute("action", "set_distribution")
        .add_attribute("sender", info.sender.to_string()))
}

// This function receives native tokens and updates the balance
pub fn receive_native(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let balance = BalanceVerified {
        native: info.funds,
        cw20: vec![],
        cw721: vec![],
    };

    receive_balance(deps, info.sender, balance)
}

// This function receives CW20 tokens and updates the balance
pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender_addr = deps.api.addr_validate(&cw20_receive_msg.sender)?;
    let cw20_balance = vec![Cw20CoinVerified {
        address: info.sender,
        amount: cw20_receive_msg.amount,
    }];

    let balance = BalanceVerified {
        native: info.funds,
        cw20: cw20_balance,
        cw721: vec![],
    };

    receive_balance(deps, sender_addr, balance)
}

// This function receives CW721 tokens and updates the balance
pub fn receive_cw721(
    deps: DepsMut,
    info: MessageInfo,
    cw721_receive_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender_addr = deps.api.addr_validate(&cw721_receive_msg.sender)?;
    let cw721_balance = vec![Cw721CollectionVerified {
        address: info.sender,
        token_ids: vec![cw721_receive_msg.token_id],
    }];

    let balance = BalanceVerified {
        native: info.funds,
        cw20: vec![],
        cw721: cw721_balance,
    };

    receive_balance(deps, sender_addr, balance)
}

fn receive_balance(
    deps: DepsMut,
    addr: Addr,
    balance: BalanceVerified,
) -> Result<Response, ContractError> {
    if !INITIAL_DUE.has(deps.storage, &addr) {
        return Err(ContractError::NoneDue {});
    }

    // Update the stored balance for the given address
    let updated_balance = BALANCE.update(deps.storage, &addr, |existing_balance| {
        existing_balance.unwrap_or_default().checked_add(&balance)
    })?;

    let due_balance = DUE.load(deps.storage, &addr)?;
    let remaining_due = due_balance.difference(&updated_balance)?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    // Handle the case where the due balance is fully paid
    if remaining_due.is_empty() {
        DUE.remove(deps.storage, &addr);

        // Lock if fully funded and send activation message if needed
        if is_fully_funded(deps.as_ref()) {
            IS_LOCKED.save(deps.storage, &true)?;

            if let Some(owner) = get_ownership(deps.storage)?.owner {
                msgs.push(CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                    contract_addr: owner.to_string(),
                    msg: to_json_binary(
                        &cw_competition::msg::ExecuteBase::<Empty, Empty>::Activate {},
                    )?,
                    funds: vec![],
                }));
            }
        }
    } else {
        DUE.save(deps.storage, &addr, &remaining_due)?;
    }

    // Update the total balance in storage
    TOTAL_BALANCE.update(deps.storage, |total| total.checked_add(&updated_balance))?;

    Ok(Response::new()
        .add_attribute("action", "receive_balance")
        .add_attribute("balance", updated_balance.to_string())
        .add_messages(msgs))
}

pub fn distribute(
    deps: DepsMut,
    info: MessageInfo,
    distribution: Vec<MemberShare<String>>,
    remainder_addr: String,
) -> Result<Response, ContractError> {
    // Ensure the sender is the owner
    assert_owner(deps.storage, &info.sender)?;

    // Ensure the contract is fully funded
    if !is_fully_funded(deps.as_ref()) {
        return Err(ContractError::NotFullyFunded {});
    }

    if !distribution.is_empty() {
        // Load the total balance available for distribution
        let total_balance = TOTAL_BALANCE.load(deps.storage)?;

        // Validate the remainder address and distribution
        let remainder_addr = deps.api.addr_validate(&remainder_addr)?;
        let validated_distribution = distribution
            .iter()
            .map(|member| member.to_validated(deps.as_ref()))
            .collect::<StdResult<_>>()?;

        // Calculate the distribution amounts based on the total balance and distribution
        let distributed_amounts = total_balance.split(&validated_distribution, &remainder_addr)?;

        // Clear the existing balance storage and update with new distribution
        BALANCE.clear(deps.storage);
        for distributed_amount in distributed_amounts {
            // Check for preset distribution and apply if available
            if let Some(preset) =
                PRESET_DISTRIBUTION.may_load(deps.storage, &distributed_amount.addr)?
            {
                let new_balances = distributed_amount
                    .balance
                    .split(&preset, &distributed_amount.addr)?;
                for new_balance in new_balances {
                    BALANCE.save(deps.storage, &new_balance.addr, &new_balance.balance)?;
                }
            } else {
                BALANCE.save(
                    deps.storage,
                    &distributed_amount.addr,
                    &distributed_amount.balance,
                )?;
            }
        }
    }

    IS_LOCKED.save(deps.storage, &false)?;

    // Clear the contract state
    DUE.clear(deps.storage);
    PRESET_DISTRIBUTION.clear(deps.storage);

    // Construct the response and return
    let keys = BALANCE
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    let response = inner_withdraw(deps, keys, None, None, true)?;
    Ok(response.add_attribute("action", "handle_competition_result"))
}

pub fn lock(deps: DepsMut, info: MessageInfo, value: bool) -> Result<Response, ContractError> {
    assert_owner(deps.storage, &info.sender)?;

    // Save the locked state to storage
    IS_LOCKED.save(deps.storage, &value)?;

    // Build and return the response
    Ok(Response::new()
        .add_attribute("action", "handle_competition_state_changed")
        .add_attribute("is_locked", value.to_string()))
}
