use std::{
    collections::{BTreeMap, BTreeSet},
    iter,
};

use arena_interface::{
    escrow::TransferEscrowOwnershipMsg,
    fees::FeeInformation,
    group::{self, MemberMsg},
};
use cosmwasm_std::{
    ensure, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, DepsMut, Empty,
    MessageInfo, Response, StdResult, Uint128,
};
use cw20::Cw20ReceiveMsg;
use cw721::receiver::Cw721ReceiveMsg;
use cw_balance::{BalanceVerified, Distribution, MemberPercentage};
use cw_ownable::{assert_owner, get_ownership};

use crate::{
    balance_manager::{BalanceManager, TotalBalanceManager},
    query::is_locked,
    state::{
        is_fully_funded, BALANCE_CW20, BALANCE_CW721, BALANCE_NATIVE, DUE_CW20, DUE_CW721,
        DUE_NATIVE, ENROLLMENT_CONTRACT, HAS_DISTRIBUTED, INITIAL_DUE_CW20, INITIAL_DUE_CW721,
        INITIAL_DUE_NATIVE, IS_LOCKED,
    },
    ContractError,
};

pub fn enrollment_withdraw(
    mut deps: DepsMut,
    info: MessageInfo,
    addrs: Vec<String>,
    entry_fee: Coin,
) -> Result<Response, ContractError> {
    ensure!(
        ENROLLMENT_CONTRACT.exists(deps.storage),
        ContractError::Unauthorized {}
    );
    ensure!(
        ENROLLMENT_CONTRACT.may_load(deps.storage)? == get_ownership(deps.storage)?.owner,
        ContractError::Unauthorized {}
    );

    let native_ref = &BALANCE_NATIVE;
    let cw20_ref = &BALANCE_CW20;
    let cw721_ref = &BALANCE_CW721;

    let balance_manager = BalanceManager::new(native_ref, cw20_ref, cw721_ref);
    let mut balance = balance_manager.load_balance(deps.as_ref(), &info.sender)?;
    let mut total_balance = TotalBalanceManager::load(deps.as_ref())?;

    let mut msgs = vec![];

    let addrs = addrs
        .into_iter()
        .map(|addr| {
            let validated_addr = deps.api.addr_validate(&addr)?; // Validate the address

            // Create and push the message for this address
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: validated_addr.to_string(),
                amount: vec![entry_fee.clone()],
            }));

            Ok(validated_addr) // Return the validated address
        })
        .collect::<StdResult<Vec<_>>>()?;

    // Deduct the full entry fee balance
    let mut native_map = BTreeMap::new();
    native_map.insert(
        entry_fee.denom.clone(),
        entry_fee
            .amount
            .checked_mul(Uint128::new(addrs.len() as u128))?,
    );

    let full_entry_fee_balance = BalanceVerified {
        native: native_map,
        cw20: BTreeMap::new(),
        cw721: BTreeMap::new(),
    };

    balance = balance.checked_sub(&full_entry_fee_balance)?;
    total_balance = total_balance.checked_sub(&full_entry_fee_balance)?;

    // Save balance
    if balance.is_empty() {
        // Clear user's balance
        balance_manager.clear_all_balances(deps.branch())?;
    } else {
        // Save updated balance
        balance_manager.save_balance(deps.branch(), &info.sender, &balance)?;
    }

    if total_balance.is_empty() {
        // Clear total balance
        TotalBalanceManager::clear(deps)?;
    } else {
        // Save updated total balance
        TotalBalanceManager::save(deps, &total_balance)?;
    }

    Ok(Response::new()
        .add_attribute("action", "enrollment_withdraw")
        .add_messages(msgs))
}

pub fn withdraw(
    mut deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Option<Binary>,
    cw721_msg: Option<Binary>,
) -> Result<Response, ContractError> {
    if is_locked(deps.as_ref()) {
        return Err(ContractError::Locked {});
    }

    let native_map = &BALANCE_NATIVE;
    let cw20_map = &BALANCE_CW20;
    let cw721_map = &BALANCE_CW721;
    let balance_manager = BalanceManager::new(native_map, cw20_map, cw721_map);

    let due_native_map = &DUE_NATIVE;
    let due_cw20_map = &DUE_CW20;
    let due_cw721_map = &DUE_CW721;
    let due_manager = BalanceManager::new(due_native_map, due_cw20_map, due_cw721_map);

    let initial_due_native_map = &INITIAL_DUE_NATIVE;
    let initial_due_cw20_map = &INITIAL_DUE_CW20;
    let initial_due_cw721_map = &INITIAL_DUE_CW721;
    let initial_due_manager = BalanceManager::new(
        initial_due_native_map,
        initial_due_cw20_map,
        initial_due_cw721_map,
    );

    // Load entire user balance
    let balance = balance_manager.load_balance(deps.as_ref(), &info.sender)?;

    // Load the total balance
    let mut total_balance = TotalBalanceManager::load(deps.as_ref())?;

    let mut msgs = vec![];

    if !balance.is_empty() {
        // Update total balance
        total_balance = total_balance.checked_sub(&balance)?;

        if !HAS_DISTRIBUTED.may_load(deps.storage)?.unwrap_or_default() {
            // Load initial due if it exists
            if let Ok(initial_due) = initial_due_manager.load_balance(deps.as_ref(), &info.sender) {
                if !initial_due.is_empty() {
                    // Set due to the initial due
                    due_manager.save_balance(deps.branch(), &info.sender, &initial_due)?;
                }
            }
        }

        // Generate messages to transmit funds
        msgs = balance.transmit_all(deps.as_ref(), &info.sender, cw20_msg, cw721_msg)?;

        // Clear user's balance
        balance_manager.clear_all_balances(deps.branch())?;
    }

    // Update or remove total balance
    if total_balance.is_empty() {
        TotalBalanceManager::clear(deps)?;
    } else {
        TotalBalanceManager::save(deps, &total_balance)?;
    }

    Ok(Response::new()
        .add_attribute("action", "withdraw")
        .add_attribute("addr", info.sender)
        .add_messages(msgs))
}

// This function receives native tokens and updates the balance
pub fn receive_native(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    if info.funds.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    let mut native_map = BTreeMap::new();
    for coin in info.funds {
        native_map.insert(coin.denom, coin.amount);
    }

    let balance = BalanceVerified {
        native: native_map,
        cw20: BTreeMap::new(),
        cw721: BTreeMap::new(),
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

    let mut cw20_map = BTreeMap::new();
    cw20_map.insert(info.sender, cw20_receive_msg.amount);

    let balance = BalanceVerified {
        native: BTreeMap::new(),
        cw20: cw20_map,
        cw721: BTreeMap::new(),
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

    let mut token_ids = BTreeSet::new();
    token_ids.insert(cw721_receive_msg.token_id);

    let mut cw721_map = BTreeMap::new();
    cw721_map.insert(info.sender, token_ids);

    let balance = BalanceVerified {
        native: BTreeMap::new(),
        cw20: BTreeMap::new(),
        cw721: cw721_map,
    };

    receive_balance(deps, sender_addr, balance)
}

fn receive_balance(
    mut deps: DepsMut,
    addr: Addr,
    balance: BalanceVerified,
) -> Result<Response, ContractError> {
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }
    if HAS_DISTRIBUTED.may_load(deps.storage)?.unwrap_or_default() {
        return Err(ContractError::AlreadyDistributed {});
    }

    let balance_native_map = &BALANCE_NATIVE;
    let balance_cw20_map = &BALANCE_CW20;
    let balance_cw721_map = &BALANCE_CW721;
    let balance_manager =
        BalanceManager::new(balance_native_map, balance_cw20_map, balance_cw721_map);

    let due_native_map = &DUE_NATIVE;
    let due_cw20_map = &DUE_CW20;
    let due_cw721_map = &DUE_CW721;
    let due_manager = BalanceManager::new(due_native_map, due_cw20_map, due_cw721_map);

    // Update the stored balance for the given address
    let existing_balance = balance_manager
        .load_balance(deps.as_ref(), &addr)
        .unwrap_or_default();
    let updated_balance = existing_balance.checked_add(&balance)?;
    balance_manager.save_balance(deps.branch(), &addr, &updated_balance)?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    // Check if the address has a due balance
    if let Ok(due_balance) = due_manager.load_balance(deps.as_ref(), &addr) {
        if !due_balance.is_empty() {
            // Calculate what's still due
            let remaining_due = due_balance.difference_to(&updated_balance)?;

            // Handle the case where the due balance is fully paid
            if remaining_due.is_empty() {
                due_manager.clear_all_balances(deps.branch())?;

                // Lock if fully funded and send activation message if needed
                if is_fully_funded(deps.as_ref()) {
                    IS_LOCKED.save(deps.storage, &true)?;

                    if let Some(owner) = get_ownership(deps.storage)?.owner {
                        msgs.push(CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                        contract_addr: owner.to_string(),
                        msg: to_json_binary(
                            &arena_interface::competition::msg::ExecuteBase::ActivateCompetition::<
                                Empty,
                                Empty,
                            > {},
                        )?,
                        funds: vec![],
                    }));
                    }
                }
            } else {
                due_manager.save_balance(deps.branch(), &addr, &remaining_due)?;
            }
        }
    }

    // Update the total balance in storage
    let total_balance = TotalBalanceManager::load(deps.as_ref()).unwrap_or_default();
    let updated_total = total_balance.checked_add(&balance)?;
    TotalBalanceManager::save(deps, &updated_total)?;

    Ok(Response::new()
        .add_attribute("action", "receive_balance")
        .add_messages(msgs))
}

pub fn distribute(
    mut deps: DepsMut,
    info: MessageInfo,
    distribution: Option<Distribution<String>>,
    layered_fees: Option<Vec<FeeInformation<String>>>,
    activation_height: Option<u64>,
    group_contract: String,
) -> Result<Response, ContractError> {
    // Ensure the sender is the owner
    assert_owner(deps.storage, &info.sender)?;

    // Validate the group contract
    let group_contract = deps.api.addr_validate(&group_contract)?;

    // Load the total balance available for distribution
    let mut total_balance = TotalBalanceManager::load(deps.as_ref()).unwrap_or_default();

    let mut msgs = vec![];
    let mut attrs = vec![];

    if !total_balance.is_empty() {
        // Process layered fees if provided
        if let Some(layered_fees) = layered_fees.as_ref() {
            // Validate the tax info
            let validated_layered_fees: Vec<FeeInformation<Addr>> = layered_fees
                .iter()
                .map(|fee| fee.into_checked(deps.as_ref()))
                .collect::<StdResult<_>>()?;

            // Process each fee
            for fee in validated_layered_fees {
                let fee_amounts = total_balance.checked_mul_floor(fee.tax)?;

                // Update total balance
                total_balance = total_balance.checked_sub(&fee_amounts)?;

                // Add messages for fee transmission if amounts are not empty
                if !fee_amounts.is_empty() {
                    msgs.extend(fee_amounts.transmit_all(
                        deps.as_ref(),
                        &fee.receiver,
                        fee.cw20_msg,
                        fee.cw721_msg,
                    )?);
                    attrs.push(("Fee", fee.receiver.to_string()));
                }
            }
        }

        // Create a distribution of all members if not provided
        let distribution = match distribution {
            Some(dist) => dist,
            None => {
                let members: Vec<MemberMsg<String>> = deps.querier.query_wasm_smart(
                    group_contract.to_string(),
                    &group::QueryMsg::Members {
                        start_after: None,
                        limit: None,
                    },
                )?;
                let percentage = Decimal::from_ratio(1u128, members.len() as u128);
                let remainder_addr = members[0].addr.clone();
                Distribution {
                    member_percentages: members
                        .into_iter()
                        .map(|x| MemberPercentage {
                            addr: x.addr,
                            percentage,
                        })
                        .collect(),
                    remainder_addr,
                }
            }
        };

        let distribution = distribution.into_checked(deps.as_ref())?;

        // Validate distribution is valid
        if !deps.querier.query_wasm_smart::<bool>(
            group_contract.to_string(),
            &group::QueryMsg::IsValidDistribution {
                addrs: distribution
                    .member_percentages
                    .iter()
                    .map(|x| x.addr.to_string())
                    .chain(iter::once(distribution.remainder_addr.to_string()))
                    .collect(),
            },
        )? {
            return Err(ContractError::InvalidDistribution {
                msg: "The distribution must contain only members of the competition".to_string(),
            });
        }

        // Calculate the distribution amounts based on the total balance and distribution
        let distributed_amounts = BalanceVerified::split(&total_balance, &distribution)?;

        // Clear existing balance storage
        let balance_native_map = &BALANCE_NATIVE;
        let balance_cw20_map = &BALANCE_CW20;
        let balance_cw721_map = &BALANCE_CW721;
        let balance_manager =
            BalanceManager::new(balance_native_map, balance_cw20_map, balance_cw721_map);
        balance_manager.clear_all_balances(deps.branch())?;

        // Query payment registry
        let payment_registry: Option<String> =
            deps.querier.query_wasm_smart(
                info.sender.to_string(),
                &arena_interface::competition::msg::QueryBase::PaymentRegistry::<
                    Empty,
                    Empty,
                    Empty,
                > {},
            )?;
        let payment_registry = payment_registry
            .map(|x| deps.api.addr_validate(&x))
            .transpose()?;

        // Process each distributed amount
        for distributed_amount in distributed_amounts {
            let mut has_preset_distribution = false;

            if let Some(ref payment_registry) = payment_registry {
                // Query preset distribution from payment registry
                let preset_distribution: Option<Distribution<String>> =
                    deps.querier.query_wasm_smart(
                        payment_registry.to_string(),
                        &arena_interface::registry::QueryMsg::GetDistribution {
                            addr: distributed_amount.addr.to_string(),
                            height: activation_height,
                        },
                    )?;

                if let Some(preset_distribution) = preset_distribution {
                    let preset_distribution = preset_distribution.into_checked(deps.as_ref())?;
                    let new_balances =
                        BalanceVerified::split(&distributed_amount.balance, &preset_distribution)?;
                    has_preset_distribution = true;

                    // Update balances based on preset distribution
                    for new_balance in new_balances {
                        let existing_balance = balance_manager
                            .load_balance(deps.as_ref(), &new_balance.addr)
                            .unwrap_or_default();
                        let updated_balance = existing_balance.checked_add(&new_balance.balance)?;
                        balance_manager.save_balance(
                            deps.branch(),
                            &new_balance.addr,
                            &updated_balance,
                        )?;
                    }
                }
            }

            if !has_preset_distribution {
                // Update balance directly if no preset distribution
                let existing_balance = balance_manager
                    .load_balance(deps.as_ref(), &distributed_amount.addr)
                    .unwrap_or_default();
                let updated_balance = existing_balance.checked_add(&distributed_amount.balance)?;
                balance_manager.save_balance(
                    deps.branch(),
                    &distributed_amount.addr,
                    &updated_balance,
                )?;
            }
        }
    }

    // Update contract state
    IS_LOCKED.save(deps.storage, &false)?;
    HAS_DISTRIBUTED.save(deps.storage, &true)?;

    // Clear all due balances
    let due_native_map = &DUE_NATIVE;
    let due_cw20_map = &DUE_CW20;
    let due_cw721_map = &DUE_CW721;
    let due_manager = BalanceManager::new(due_native_map, due_cw20_map, due_cw721_map);
    due_manager.clear_all_balances(deps.branch())?;

    // Update total balance
    TotalBalanceManager::save(deps, &total_balance)?;

    Ok(Response::new()
        .add_attribute("action", "distribute")
        .add_attributes(attrs)
        .add_messages(msgs))
}

pub fn lock(
    deps: DepsMut,
    info: MessageInfo,
    value: bool,
    transfer_ownership: Option<TransferEscrowOwnershipMsg>,
) -> Result<Response, ContractError> {
    assert_owner(deps.storage, &info.sender)?;

    // Save the locked state to storage
    IS_LOCKED.save(deps.storage, &value)?;

    let mut res = Response::new()
        .add_attribute("action", "lock")
        .add_attribute("is_locked", value.to_string());

    // Set new owner if provided
    if let Some(new_ownership) = transfer_ownership {
        let ownership =
            cw_ownable::initialize_owner(deps.storage, deps.api, Some(&new_ownership.addr))?;
        res = res.add_attributes(ownership.into_attributes());
    }

    Ok(res)
}

pub fn claw(mut deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Get the direct owner of this contract
    let owner = get_ownership(deps.storage)?.owner;

    // Ensure owner exists
    let owner_addr = owner.ok_or(ContractError::Unauthorized {})?;

    // Query the owner of the owner (DAO) using cw_ownable
    let dao_addr: Addr = deps.querier.query_wasm_smart(
        owner_addr.to_string(),
        &arena_interface::competition::msg::QueryBase::<Empty, Empty, Empty>::DAO {},
    )?;

    // Verify that the sender is the DAO
    if info.sender != dao_addr {
        return Err(ContractError::Unauthorized {});
    }

    // Load the total balance to be clawed back
    let total_balance = TotalBalanceManager::load(deps.as_ref())?;
    if total_balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    // Generate messages to transmit all balances to the DAO
    let msgs = total_balance.transmit_all(
        deps.as_ref(),
        &dao_addr,
        None, // No special cw20 message
        None, // No special cw721 message
    )?;

    // Clear all balances
    BalanceManager::new(&BALANCE_NATIVE, &BALANCE_CW20, &BALANCE_CW721)
        .clear_all_balances(deps.branch())?;
    TotalBalanceManager::clear(deps)?;

    Ok(Response::new()
        .add_attribute("action", "claw")
        .add_attribute("dao", dao_addr.to_string())
        .add_messages(msgs))
}
