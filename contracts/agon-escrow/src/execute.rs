use cosmwasm_std::{
    Addr, Attribute, Binary, Decimal, Deps, DepsMut, MessageInfo, Response, StdResult,
};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw_balance::{
    convert_native_funds, get_validated_distribution, Balance, Distribution, DistributionRaw,
};
use std::collections::{BTreeMap, HashMap};

use crate::{
    query::is_locked,
    state::{
        get_distributable_balance, ADMIN, BALANCE, DISTRIBUTION, DUE, IS_LOCKED, STAKE,
        TOTAL_BALANCE, TOTAL_STAKE,
    },
    ContractError,
};

// This function refunds the balance of given addresses
fn inner_withdraw(
    deps: DepsMut,
    addrs: Vec<Addr>,
    cw20_msg: Option<Binary>,
    cw721_msg: Option<Binary>,
) -> Result<Response, ContractError> {
    // Load the key and total_balance from storage
    let mut total_balance = TOTAL_BALANCE.load(deps.storage)?;
    let mut msgs = vec![];
    let mut attrs = vec![];

    for addr in addrs {
        // Load the balance of the current address
        let balance = BALANCE.may_load(deps.storage, &addr)?;

        // If the balance is empty, skip this address
        if balance.is_none() || balance.as_ref().unwrap().is_empty() {
            continue;
        }

        // Prepare messages for the balance transmit
        msgs.append(&mut balance.as_ref().unwrap().transmit(
            deps.as_ref(),
            &addr,
            cw20_msg.clone(),
            cw721_msg.clone(),
        )?);

        // Add address as an attribute to the response
        attrs.push(Attribute {
            key: "addr".to_string(),
            value: addr.to_string(),
        });

        // Update the total_balance by subtracting the refunded balance
        total_balance = total_balance.checked_sub(&balance.unwrap())?;
    }

    // Save the updated total_balance to storage
    TOTAL_BALANCE.save(deps.storage, &total_balance)?;

    // Build and return the response
    Ok(Response::new()
        .add_attribute("action", "refund")
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

    inner_withdraw(deps, vec![info.sender], cw20_msg, cw721_msg)
}

/// Sets the distribution for the sender based on the provided distribution map.
///
/// # Arguments
///
/// * `deps` - A mutable reference to the contract's dependencies.
/// * `info` - The information about the sender and funds.
/// * `distribution` - The distribution map with keys as addresses in string format and values as Uint128.
///
/// # Returns
///
/// * `Result<Response, ContractError>` - A result containing a response or a contract error.
pub fn set_distribution(
    deps: DepsMut,
    info: MessageInfo,
    distribution: DistributionRaw,
) -> Result<Response, ContractError> {
    // Convert String keys to Addr
    let validated_distribution = distribution
        .into_iter()
        .map(|(k, v)| {
            let addr = deps.api.addr_validate(&k)?;
            Ok((addr, v))
        })
        .collect::<StdResult<BTreeMap<Addr, Decimal>>>()?;

    // Save distribution in the state
    DISTRIBUTION.save(deps.storage, &info.sender, &validated_distribution)?;

    Ok(Response::new()
        .add_attribute("action", "set_distribution")
        .add_attribute("sender", info.sender.to_string()))
}

// This function receives native tokens and updates the balance
pub fn receive_native(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let balance = Balance {
        native: convert_native_funds(&info.funds),
        cw20: HashMap::new(),
        cw721: HashMap::new(),
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
    let mut cw20_balance = HashMap::new();
    cw20_balance.insert(sender_addr.clone(), cw20_receive_msg.amount);

    let balance = Balance {
        native: convert_native_funds(&info.funds),
        cw20: cw20_balance,
        cw721: HashMap::new(),
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
    let mut cw721_balance = HashMap::new();
    cw721_balance.insert(sender_addr.clone(), vec![cw721_receive_msg.token_id]);

    let balance = Balance {
        native: convert_native_funds(&info.funds),
        cw20: HashMap::new(),
        cw721: cw721_balance,
    };

    receive_balance(deps, sender_addr, balance)
}

// This function updates the balance
fn receive_balance(deps: DepsMut, addr: Addr, balance: Balance) -> Result<Response, ContractError> {
    // Update the balance in storage for the given address
    BALANCE.update(deps.storage, &addr, |x| -> StdResult<_> {
        if x.is_none() {
            return Ok(balance.clone());
        } else {
            return Ok(balance.checked_add(&x.as_ref().unwrap())?);
        }
    })?;

    // Update the due balance in storage for the given address
    DUE.update(deps.storage, &addr, |x| -> StdResult<_> {
        if x.is_none() {
            return Ok(Balance::default());
        } else {
            return Ok(x.as_ref().unwrap().checked_sub(&balance)?);
        }
    })?;

    // Update the total balance in storage
    TOTAL_BALANCE.update(deps.storage, |x| -> StdResult<_> {
        Ok(x.checked_add(&balance)?) //do not factor in stake amount
    })?;

    // Build and return the response
    Ok(Response::new()
        .add_attribute("action", "receive_balance")
        .add_attribute("balance", balance.to_string()))
}

pub fn apply_preset_distribution(
    deps: Deps,
    input_distribution: Distribution,
) -> Result<Distribution, ContractError> {
    let mut output_distribution = Distribution::new();

    for (addr, input_weight) in input_distribution {
        // Check if there's a preset distribution for the given address
        if let Some(preset_distribution) =
            DISTRIBUTION.may_load(deps.storage, &addr).unwrap_or(None)
        {
            let preset_total = preset_distribution
                .values()
                .cloned()
                .try_fold(Decimal::zero(), |accumulator, x| accumulator.checked_add(x))?;

            for (preset_addr, preset_weight) in preset_distribution {
                let new_weight = preset_weight
                    .checked_mul(input_weight)?
                    .checked_div(preset_total)?;
                output_distribution.insert(preset_addr, new_weight);
            }
        } else {
            output_distribution.insert(addr, input_weight);
        }
    }

    Ok(output_distribution)
}

// This function handles the competition result message.
pub fn distribute(
    deps: DepsMut,
    info: MessageInfo,
    distribution: DistributionRaw,
    remainder_addr: String,
) -> Result<Response, ContractError> {
    // Assert that the sender is an admin.
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    // Calculate the distributable balance.
    let distributable_total = get_distributable_balance(deps.as_ref())?;

    // Validate the remainder address.
    let remainder_addr = deps.api.addr_validate(&remainder_addr)?;

    // Validate the provided distribution.
    let validated_distribution = get_validated_distribution(deps.as_ref(), &distribution)?;

    // Apply the preset distribution, if any.
    let validated_distribution = apply_preset_distribution(deps.as_ref(), validated_distribution)?;

    // Calculate the splits based on the distributable total and the validated distribution.
    let distributed_amounts =
        distributable_total.split(&validated_distribution, &remainder_addr)?;

    // Retrieve the current stakes.
    let stakes = STAKE
        .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .collect::<StdResult<HashMap<Addr, Balance>>>()?;

    // Clear the existing balance storage.
    BALANCE.clear(deps.storage);

    // Save the new balances based on the calculated splits.
    for (addr, balance) in distributed_amounts {
        BALANCE.save(deps.storage, &addr, &balance)?;
    }

    // Update the balances with the stakes.
    for (addr, balance) in stakes {
        BALANCE.update(deps.storage, &addr, |x| -> Result<Balance, ContractError> {
            Ok(match x {
                Some(value) => balance.checked_add(&value)?,
                None => balance,
            })
        })?;
    }

    // Remove the total stake from storage.
    TOTAL_STAKE.remove(deps.storage);

    // Return the response with the added action attribute.
    Ok(Response::new().add_attribute("action", "handle_competition_result"))
}

// This function handles the competition state change message
pub fn lock(deps: DepsMut, info: MessageInfo, value: bool) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    // Save the locked state to storage
    IS_LOCKED.save(deps.storage, &value)?;

    // Build and return the response
    Ok(Response::new()
        .add_attribute("action", "handle_competition_state_changed")
        .add_attribute("is_locked", value.to_string()))
}
