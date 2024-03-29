use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, DepsMut, Empty, MessageInfo, Response, StdResult,
};
use cw20::{Cw20CoinVerified, Cw20ReceiveMsg};
use cw721::Cw721ReceiveMsg;
use cw_balance::{BalanceVerified, Cw721CollectionVerified, Distribution};
use cw_competition::escrow::TaxInformation;
use cw_ownable::{assert_owner, get_ownership};

use crate::{
    query::is_locked,
    state::{
        is_fully_funded, BALANCE, DUE, HAS_DISTRIBUTED, INITIAL_DUE, IS_LOCKED,
        PRESET_DISTRIBUTION, TAX_AT_WITHDRAWAL, TOTAL_BALANCE,
    },
    ContractError,
};

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Option<Binary>,
    cw721_msg: Option<Binary>,
) -> Result<Response, ContractError> {
    if is_locked(deps.as_ref()) {
        return Err(ContractError::Locked {});
    }

    // Initialize total_balance based on processing status
    let mut total_balance = TOTAL_BALANCE.may_load(deps.storage)?.unwrap_or_default();

    // Load and process balance for each address
    let msgs = if let Some(mut balance) = BALANCE.may_load(deps.storage, &info.sender)? {
        if balance.is_empty() {
            return Err(ContractError::EmptyBalance {});
        }

        // If the total balance has already been taxed, then deduct at the individual level
        if let Some(tax) = TAX_AT_WITHDRAWAL.may_load(deps.storage)? {
            balance = balance.checked_sub(&balance.checked_mul_floor(tax)?)?;
        }

        // Update total balance and related storage entries
        BALANCE.remove(deps.storage, &info.sender);
        total_balance = total_balance.checked_sub(&balance)?;

        if !HAS_DISTRIBUTED.load(deps.storage)? {
            // Set due to the initial due
            let initial_due = &INITIAL_DUE.load(deps.storage, &info.sender)?;
            DUE.save(deps.storage, &info.sender, initial_due)?;
        }

        // Update or remove total balance
        if total_balance.is_empty() {
            TOTAL_BALANCE.remove(deps.storage);
        } else {
            TOTAL_BALANCE.save(deps.storage, &total_balance)?;
        }

        balance.transmit_all(
            deps.as_ref(),
            &info.sender,
            cw20_msg.clone(),
            cw721_msg.clone(),
        )?
    } else {
        vec![]
    };

    Ok(Response::new()
        .add_attribute("action", "withdraw")
        .add_attribute("addr", info.sender)
        .add_messages(msgs))
}

pub fn set_distribution(
    deps: DepsMut,
    info: MessageInfo,
    distribution: Option<Distribution<String>>,
) -> Result<Response, ContractError> {
    if let Some(distribution) = &distribution {
        // Validate
        let distribution = distribution.into_checked(deps.as_ref())?;

        // Save distribution in the state
        PRESET_DISTRIBUTION.save(deps.storage, &info.sender, &distribution)?;
    } else {
        PRESET_DISTRIBUTION.remove(deps.storage, &info.sender);
    }

    Ok(Response::new()
        .add_attribute("action", "set_distribution")
        .add_attribute(
            "distribution",
            distribution
                .map(|some| some.to_string())
                .unwrap_or("None".to_owned()),
        ))
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
        return Err(ContractError::InvalidDue {
            msg: "User is not a participant".to_string(),
        });
    }

    // Update the stored balance for the given address
    let updated_balance =
        BALANCE.update(deps.storage, &addr, |maybe_balance| match maybe_balance {
            Some(existing_balance) => existing_balance.checked_add(&balance),
            None => Ok(balance),
        })?;

    let due_balance = DUE.load(deps.storage, &addr)?;
    let remaining_due = updated_balance.difference(&due_balance)?;

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
    if TOTAL_BALANCE.exists(deps.storage) {
        TOTAL_BALANCE.update(deps.storage, |total| total.checked_add(&updated_balance))?;
    } else {
        TOTAL_BALANCE.save(deps.storage, &updated_balance)?;
    }

    Ok(Response::new()
        .add_attribute("action", "receive_balance")
        .add_attribute("balance", updated_balance.to_string())
        .add_messages(msgs))
}

pub fn distribute(
    deps: DepsMut,
    info: MessageInfo,
    distribution: Option<Distribution<String>>,
    tax_info: Option<TaxInformation<String>>,
) -> Result<Response, ContractError> {
    // Ensure the sender is the owner
    assert_owner(deps.storage, &info.sender)?;

    // Load the total balance available for distribution
    let mut total_balance = TOTAL_BALANCE.load(deps.storage)?;

    // Validate the tax info
    let validated_tax_info = tax_info
        .as_ref()
        .map(|tax_info| tax_info.into_checked(deps.as_ref()))
        .transpose()?;

    // Process the tax
    // This will automatically be sent to the receiver
    let msgs = if let Some(tax_info) = validated_tax_info {
        let tax = total_balance.checked_mul_floor(tax_info.tax)?;

        total_balance =
            TOTAL_BALANCE.update(deps.storage, |x| -> StdResult<_> { x.checked_sub(&tax) })?;

        // If funds are not split, then we should have the tax at withdrawal
        if distribution.is_none() {
            TAX_AT_WITHDRAWAL.save(deps.storage, &tax_info.tax)?;
        }

        if !tax.is_empty() {
            tax.transmit_all(
                deps.as_ref(),
                &tax_info.receiver,
                tax_info.cw20_msg,
                tax_info.cw721_msg,
            )?
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Clear the existing balance storage and update with new distribution
    if let Some(distribution) = &distribution {
        let distribution = distribution.into_checked(deps.as_ref())?;

        // Calculate the distribution amounts based on the total balance and distribution
        let distributed_amounts = total_balance.split(&distribution)?;

        BALANCE.clear(deps.storage);
        for distributed_amount in distributed_amounts {
            // Check for preset distribution and apply if available
            if let Some(preset) =
                PRESET_DISTRIBUTION.may_load(deps.storage, &distributed_amount.addr)?
            {
                let new_balances = distributed_amount.balance.split(&preset)?;
                for new_balance in new_balances {
                    BALANCE.update(
                        deps.storage,
                        &new_balance.addr,
                        |old_balance| -> Result<_, ContractError> {
                            match old_balance {
                                Some(old_balance) => {
                                    Ok(old_balance.checked_add(&new_balance.balance)?)
                                }
                                None => Ok(new_balance.balance),
                            }
                        },
                    )?;
                }
            } else {
                BALANCE.update(
                    deps.storage,
                    &distributed_amount.addr,
                    |old_balance| -> Result<_, ContractError> {
                        match old_balance {
                            Some(old_balance) => {
                                Ok(old_balance.checked_add(&distributed_amount.balance)?)
                            }
                            None => Ok(distributed_amount.balance),
                        }
                    },
                )?;
            }
        }
    }

    IS_LOCKED.save(deps.storage, &false)?;
    HAS_DISTRIBUTED.save(deps.storage, &true)?;

    // Clear the contract state
    DUE.clear(deps.storage);
    PRESET_DISTRIBUTION.clear(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "handle_competition_result")
        .add_attribute(
            "tax",
            tax_info
                .map(|some| some.tax.to_string())
                .unwrap_or("None".to_owned()),
        )
        .add_messages(msgs))
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
