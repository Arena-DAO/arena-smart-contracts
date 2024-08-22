use arena_interface::competition::msg::{
    EscrowInstantiateInfo, ExecuteBaseFns as _, QueryBaseFns as _,
};
use arena_interface::escrow::ExecuteMsgFns as _;
use arena_wager_module::msg::WagerInstantiateExt;
use cosmwasm_std::{coins, to_json_binary, Addr, Coin, Decimal, Uint128};
use cw_balance::{BalanceUnchecked, Distribution, MemberBalanceUnchecked, MemberPercentage};
use cw_orch::{anyhow, prelude::*};
use cw_utils::Expiration;

use crate::Arena;

use super::{ADMIN, DENOM, PREFIX};

#[test]
fn test_create_wager() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let mut arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;

    let user1 = mock.addr_make("user1");
    let user2 = mock.addr_make("user2");

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager
    let res = arena.arena_wager_module.create_competition(
        "A test wager".to_string(),
        Expiration::AtHeight(1000000),
        WagerInstantiateExt {
            registered_members: None,
        },
        "Test Wager".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: vec![
                    MemberBalanceUnchecked {
                        addr: user1.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: user2.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                ],
                should_activate_on_funded: None,
            })?,
            label: "Wager Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Wager Rule".to_string()]),
        None,
        None,
    )?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "create_competition")));

    // Query the created wager
    let wager = arena.arena_wager_module.competition(Uint128::one())?;
    assert_eq!(wager.name, "Test Wager");

    Ok(())
}

#[test]
fn test_process_wager() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let mut arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager
    let res = arena.arena_wager_module.create_competition(
        "A test wager".to_string(),
        Expiration::AtHeight(1000000),
        WagerInstantiateExt {
            registered_members: None,
        },
        "Test Wager".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: vec![
                    MemberBalanceUnchecked {
                        addr: user1.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: user2.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                ],
                should_activate_on_funded: None,
            })?,
            label: "Wager Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Wager Rule".to_string()]),
        None,
        None,
    )?;

    let escrow_addr = res
        .events
        .iter()
        .find_map(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "escrow_addr")
                .map(|attr| attr.value.clone())
        })
        .unwrap();

    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr));

    // Fund the escrow
    arena.arena_escrow.set_sender(&user1);
    arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    arena.arena_escrow.set_sender(&user2);
    arena.arena_escrow.receive_native(&coins(1000, DENOM))?;

    // Process the wager
    arena.arena_wager_module.process_competition(
        Uint128::one(),
        Some(Distribution {
            member_percentages: vec![MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::one(),
            }],
            remainder_addr: user1.to_string(),
        }),
    )?;

    // Check the result
    let result = arena.arena_wager_module.result(Uint128::one())?;
    assert!(result.is_some());

    // Withdraw
    arena.arena_escrow.call_as(&user1).withdraw(None, None)?;

    // Check balances
    let user1_balance = mock.query_balance(&user1, DENOM)?;
    let user2_balance = mock.query_balance(&user2, DENOM)?;
    assert_eq!(user1_balance, Uint128::new(10900)); // Initial 10000 - 1000 stake + 1900 winnings (after 5% tax)
    assert_eq!(user2_balance, Uint128::new(9000)); // Initial 10000 - 1000 stake

    Ok(())
}

#[test]
fn test_wager_with_additional_fees() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let mut arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;
    let fee_receiver = mock.addr_make("fee_receiver");

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager with additional fees
    let res = arena.arena_wager_module.create_competition(
        "Wager with fees".to_string(),
        Expiration::AtHeight(mock.block_info()?.height + 100),
        WagerInstantiateExt {
            registered_members: None,
        },
        "Fee Wager".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: vec![
                    MemberBalanceUnchecked {
                        addr: user1.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: user2.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                ],
                should_activate_on_funded: None,
            })?,
            label: "Fee Wager Escrow".to_string(),
            additional_layered_fees: Some(vec![arena_interface::fees::FeeInformation {
                tax: Decimal::percent(2),
                receiver: fee_receiver.to_string(),
                cw20_msg: None,
                cw721_msg: None,
            }]),
        }),
        None,
        Some(vec!["Fee Wager Rule".to_string()]),
        None,
        None,
    )?;

    let escrow_addr = res
        .events
        .iter()
        .find_map(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "escrow_addr")
                .map(|attr| attr.value.clone())
        })
        .unwrap();

    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr));

    // Fund the escrow
    arena.arena_escrow.set_sender(&user1);
    arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    arena.arena_escrow.set_sender(&user2);
    arena.arena_escrow.receive_native(&coins(1000, DENOM))?;

    // Process the wager
    arena.arena_wager_module.process_competition(
        Uint128::one(),
        Some(Distribution {
            member_percentages: vec![MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::one(),
            }],
            remainder_addr: user1.to_string(),
        }),
    )?;

    // Withdraw
    arena.arena_escrow.call_as(&user1).withdraw(None, None)?;

    // Check balances
    let user1_balance = mock.query_balance(&user1, DENOM)?;
    let user2_balance = mock.query_balance(&user2, DENOM)?;
    let fee_receiver_balance = mock.query_balance(&fee_receiver, DENOM)?;
    let dao_balance = mock.query_balance(&arena.dao_dao.dao_core.address()?, DENOM)?; // Assuming admin is the DAO in this case

    assert_eq!(user1_balance, Uint128::new(10862)); // Initial 10000 - 1000 stake + 1862 winnings (after 5% tax and 2% additional fee)
    assert_eq!(user2_balance, Uint128::new(9000)); // Initial 10000 - 1000 stake
    assert_eq!(fee_receiver_balance, Uint128::new(38)); // 2% of 1900
    assert_eq!(dao_balance, Uint128::new(100)); // 5% of 2000

    Ok(())
}
