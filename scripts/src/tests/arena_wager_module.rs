use arena_interface::competition::msg::{
    EscrowInstantiateInfo, ExecuteBaseFns as _, QueryBaseFns as _,
};
use arena_interface::competition::state::CompetitionStatus;
use arena_interface::competition::stats::{
    MemberStatsMsg, StatAggregationType, StatMsg, StatType, StatValue, StatValueType,
};
use arena_interface::core::QueryExtFns;
use arena_interface::escrow::{ExecuteMsgFns as _, QueryMsgFns as _};
use arena_interface::group::{self, GroupContractInfo};
use arena_interface::registry::ExecuteMsgFns as _;
use arena_wager_module::msg::{MigrateMsg, WagerInstantiateExt};
use cosmwasm_std::{coins, to_json_binary, Addr, Coin, Decimal, Uint128};
use cw_balance::{
    BalanceUnchecked, BalanceVerified, Distribution, MemberBalanceUnchecked, MemberPercentage,
};
use cw_orch::{anyhow, prelude::*};
use cw_orch_clone_testing::CloneTesting;
use cw_utils::Expiration;
use dao_interface::state::ModuleInstantiateInfo;
use dao_interface::CoreQueryMsgFns;
use networks::PION_1;

use crate::arena::Arena;
use crate::tests::helpers::{setup_arena, setup_voting_module, teams_to_members};

use super::{DENOM, PREFIX};

#[test]
fn test_create_wager() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make("user1");
    let user2 = mock.addr_make("user2");

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager
    let res = arena.arena_wager_module.create_competition(
        "A test wager".to_string(),
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
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
            })?,
            label: "Wager Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Wager Rule".to_string()]),
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
    let (mut arena, admin) = setup_arena(&mock)?;
    setup_voting_module(
        &mock,
        &arena,
        vec![cw4::Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    )?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager
    let res = arena.arena_wager_module.create_competition(
        "A test wager".to_string(),
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
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
            })?,
            label: "Wager Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Wager Rule".to_string()]),
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

    // Ensure ELO was updated
    let user1_rating = arena.arena_core.rating(user1.to_string(), Uint128::one())?;
    let user2_rating = arena.arena_core.rating(user2.to_string(), Uint128::one())?;

    assert!(user1_rating.is_some());
    assert!(user2_rating.is_some());

    assert!(user1_rating.as_ref().unwrap().value > user2_rating.as_ref().unwrap().value);

    // Check that ELO for category 2 is different
    assert_ne!(
        user1_rating,
        arena
            .arena_core
            .rating(user1.to_string(), Uint128::new(2))?
    );

    Ok(())
}

#[test]
fn test_wager_with_additional_fees() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;
    let fee_receiver = mock.addr_make("fee_receiver");

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager with additional fees
    let res = arena.arena_wager_module.create_competition(
        "Wager with fees".to_string(),
        Expiration::AtHeight(mock.block_info()?.height + 100),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
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

#[test]
fn test_wager_with_preset_distributions() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;
    let user3 = mock.addr_make_with_balance("user3", coins(10000, DENOM))?;

    // Set preset distributions for user1 and user2
    let distribution1 = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: user3.to_string(),
                percentage: Decimal::percent(20),
            },
            MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::percent(80),
            },
        ],
        remainder_addr: user1.to_string(),
    };
    arena
        .arena_payment_registry
        .call_as(&user1)
        .set_distribution(distribution1)?;

    let distribution2 = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: user3.to_string(),
                percentage: Decimal::percent(30),
            },
            MemberPercentage {
                addr: user2.to_string(),
                percentage: Decimal::percent(70),
            },
        ],
        remainder_addr: user2.to_string(),
    };
    arena
        .arena_payment_registry
        .call_as(&user2)
        .set_distribution(distribution2)?;

    // Create a wager
    arena.arena_wager_module.set_sender(&admin);
    let res = arena.arena_wager_module.create_competition(
        "Wager with preset distributions".to_string(),
        Expiration::AtHeight(mock.block_info()?.height + 100),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone(), user3.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
        "Preset Distribution Wager".to_string(),
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
            })?,
            label: "Preset Distribution Wager Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Preset Distribution Wager Rule".to_string()]),
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

    // Advance the block
    mock.next_block()?;

    // Fund the escrow
    arena
        .arena_escrow
        .call_as(&user1)
        .receive_native(&coins(1000, DENOM))?;
    arena
        .arena_escrow
        .call_as(&user2)
        .receive_native(&coins(1000, DENOM))?;

    // Process the wager
    arena.arena_wager_module.process_competition(
        Uint128::one(),
        Some(Distribution {
            member_percentages: vec![
                MemberPercentage {
                    addr: user1.to_string(),
                    percentage: Decimal::percent(70),
                },
                MemberPercentage {
                    addr: user2.to_string(),
                    percentage: Decimal::percent(30),
                },
            ],
            remainder_addr: user1.to_string(),
        }),
    )?;

    // Check escrow balances
    let user1_balance = arena.arena_escrow.balance(user1.to_string())?;
    let user2_balance = arena.arena_escrow.balance(user2.to_string())?;
    let user3_balance = arena.arena_escrow.balance(user3.to_string())?;

    // Calculate expected balances
    // Total pot: 2000
    // DAO fee: 5% of 2000 = 100
    // Remaining: 1900
    // user1 gets 70% of 1900 = 1330
    // user2 gets 30% of 1900 = 570
    // user3 gets 20% of user1's winnings (266) and 30% of user2's winnings (171) = 437

    let expected_user1_balance = BalanceVerified {
        native: Some(coins(1064, DENOM)), // 1330 - 266 (20% to user3)
        cw20: None,
        cw721: None,
    };
    let expected_user2_balance = BalanceVerified {
        native: Some(coins(399, DENOM)), // 570 - 171 (30% to user3)
        cw20: None,
        cw721: None,
    };
    let expected_user3_balance = BalanceVerified {
        native: Some(coins(437, DENOM)), // 266 from user1 + 171 from user2
        cw20: None,
        cw721: None,
    };

    assert_eq!(user1_balance, Some(expected_user1_balance));
    assert_eq!(user2_balance, Some(expected_user2_balance));
    assert_eq!(user3_balance, Some(expected_user3_balance));

    // Check DAO balance
    let dao_balance = mock.query_balance(&arena.dao_dao.dao_core.address()?, DENOM)?;
    assert_eq!(dao_balance, Uint128::new(100)); // 5% of 2000

    Ok(())
}

#[test]
fn test_wager_with_draw() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;

    // Create a wager
    arena.arena_wager_module.set_sender(&admin);
    let res = arena.arena_wager_module.create_competition(
        "Wager".to_string(),
        Expiration::AtHeight(mock.block_info()?.height + 100),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
        "Wager".to_string(),
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
            })?,
            label: "Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
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

    // Advance the block
    mock.next_block()?;

    // Fund the escrow
    arena
        .arena_escrow
        .call_as(&user1)
        .receive_native(&coins(1000, DENOM))?;
    arena
        .arena_escrow
        .call_as(&user2)
        .receive_native(&coins(1000, DENOM))?;

    // Process the wager
    arena
        .arena_wager_module
        .process_competition(Uint128::one(), None)?;

    // Check escrow balances
    let user1_balance = arena.arena_escrow.balance(user1.to_string())?;
    let user2_balance = arena.arena_escrow.balance(user2.to_string())?;

    // Calculate expected balances
    // Total pot: 2000
    // DAO fee: 5% of 2000 = 100
    // Remaining: 1900
    // user1 gets 1000 * 95%  = 950
    // user2 gets 1000 * 95%  = 950

    let expected_user1_balance = BalanceVerified {
        native: Some(coins(950, DENOM)), // 1330 - 266 (20% to user3)
        cw20: None,
        cw721: None,
    };
    let expected_user2_balance = BalanceVerified {
        native: Some(coins(950, DENOM)), // 570 - 171 (30% to user3)
        cw20: None,
        cw721: None,
    };

    assert_eq!(user1_balance, Some(expected_user1_balance));
    assert_eq!(user2_balance, Some(expected_user2_balance));

    // Check DAO balance
    let dao_balance = mock.query_balance(&arena.dao_dao.dao_core.address()?, DENOM)?;
    assert_eq!(dao_balance, Uint128::new(100)); // 5% of 2000

    Ok(())
}

#[test]
fn test_wager_with_malicious_host() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;

    // Create a wager
    arena.arena_wager_module.set_sender(&admin);
    let res = arena.arena_wager_module.create_competition(
        "Wager".to_string(),
        Expiration::AtHeight(mock.block_info()?.height + 100),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
        "Wager".to_string(),
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
            })?,
            label: "Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
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

    // Advance the block
    mock.next_block()?;

    // Fund the escrow
    arena
        .arena_escrow
        .call_as(&user1)
        .receive_native(&coins(1000, DENOM))?;
    arena
        .arena_escrow
        .call_as(&user2)
        .receive_native(&coins(1000, DENOM))?;

    // Process the wager
    // The host is attempting to claim all of the money
    let result = arena.arena_wager_module.process_competition(
        Uint128::one(),
        Some(Distribution {
            member_percentages: vec![MemberPercentage {
                addr: admin.to_string(),
                percentage: Decimal::one(),
            }],
            remainder_addr: admin.to_string(),
        }),
    );

    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_wager_with_updated_distribution_after_activation() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;
    let user3 = mock.addr_make_with_balance("user3", coins(10000, DENOM))?;

    // Set initial distribution for user1
    let initial_distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: user3.to_string(),
                percentage: Decimal::percent(20),
            },
            MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::percent(80),
            },
        ],
        remainder_addr: user1.to_string(),
    };
    arena
        .arena_payment_registry
        .call_as(&user1)
        .set_distribution(initial_distribution)?;

    // Create a wager
    arena.arena_wager_module.set_sender(&admin);
    let res = arena.arena_wager_module.create_competition(
        "Wager with updated distribution".to_string(),
        Expiration::AtHeight(mock.block_info()?.height + 100),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
        "Updated Distribution Wager".to_string(),
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
            })?,
            label: "Updated Distribution Wager Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Updated Distribution Wager Rule".to_string()]),
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

    // Advance the block
    mock.next_block()?;

    // Fund the escrow
    arena
        .arena_escrow
        .call_as(&user1)
        .receive_native(&coins(1000, DENOM))?;
    arena
        .arena_escrow
        .call_as(&user2)
        .receive_native(&coins(1000, DENOM))?;

    // Update distribution for user1 after the wager is fully funded
    let updated_distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: user3.to_string(),
                percentage: Decimal::percent(50),
            },
            MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::percent(50),
            },
        ],
        remainder_addr: user1.to_string(),
    };
    arena
        .arena_payment_registry
        .call_as(&user1)
        .set_distribution(updated_distribution)?;
    mock.next_block()?;

    // Process the wager
    arena.arena_wager_module.process_competition(
        Uint128::one(),
        Some(Distribution {
            member_percentages: vec![MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::percent(100),
            }],
            remainder_addr: user1.to_string(),
        }),
    )?;

    // Check escrow balances
    let user1_balance = arena.arena_escrow.balance(user1.to_string())?;
    let user2_balance = arena.arena_escrow.balance(user2.to_string())?;
    let user3_balance = arena.arena_escrow.balance(user3.to_string())?;

    // Calculate expected balances
    // Total pot: 2000
    // DAO fee: 5% of 2000 = 100
    // Remaining: 1900
    // user1 gets 100% of 1900 = 1900
    // user3 gets 20% of user1's winnings (380) based on the initial distribution

    let expected_user1_balance = BalanceVerified {
        native: Some(coins(1520, DENOM)), // 1900 - 380 (20% to user3)
        cw20: None,
        cw721: None,
    };
    let expected_user3_balance = BalanceVerified {
        native: Some(coins(380, DENOM)), // 20% of 1900
        cw20: None,
        cw721: None,
    };

    assert_eq!(user1_balance, Some(expected_user1_balance));
    assert_eq!(user2_balance, None);
    assert_eq!(user3_balance, Some(expected_user3_balance));

    // Check DAO balance
    let dao_balance = mock.query_balance(&arena.dao_dao.dao_core.address()?, DENOM)?;
    assert_eq!(dao_balance, Uint128::new(100)); // 5% of 2000

    Ok(())
}

#[test]
fn test_jailed_wager_resolved_by_dao() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;
    setup_voting_module(
        &mock,
        &arena,
        vec![cw4::Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    )?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager
    let res = arena.arena_wager_module.create_competition(
        "A test wager".to_string(),
        Expiration::AtHeight(100000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
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
            })?,
            label: "Wager Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Wager Rule".to_string()]),
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
    let activation_height = mock.block_info()?.height;

    // Jailing before expiration is an error
    arena.arena_wager_module.set_sender(&admin);
    let res = arena.arena_wager_module.jail_competition(
        Uint128::one(),
        "Jailed Wager".to_string(),
        "This wager needs DAO resolution".to_string(),
        Some(Distribution {
            member_percentages: vec![MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::one(),
            }],
            remainder_addr: user1.to_string(),
        }),
        &[],
    );
    assert!(res.is_err());

    // Wait enough time for the wager to be jailable
    mock.wait_blocks(100000)?;

    // Jail the wager
    arena.arena_wager_module.jail_competition(
        Uint128::one(),
        "Jailed Wager".to_string(),
        "This wager needs DAO resolution".to_string(),
        Some(Distribution {
            member_percentages: vec![MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::one(),
            }],
            remainder_addr: user1.to_string(),
        }),
        &[],
    )?;

    // Ensure other person can propose a result
    arena.arena_wager_module.call_as(&user1).jail_competition(
        Uint128::one(),
        "Jailed Wager".to_string(),
        "This wager needs DAO resolution".to_string(),
        Some(Distribution {
            member_percentages: vec![MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::one(),
            }],
            remainder_addr: user1.to_string(),
        }),
        &[],
    )?;

    // Check that the wager is jailed
    let wager = arena.arena_wager_module.competition(Uint128::one())?;
    assert_eq!(
        wager.status,
        CompetitionStatus::Jailed { activation_height }
    );

    // Execute the jailed proposal after expiration
    mock.wait_blocks(100)?;
    mock.call_as(&admin).execute(
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1 },
        &[],
        &arena
            .dao_dao
            .dao_core
            .proposal_modules(None, None)?
            .iter()
            .find(|x| x.prefix == "B")
            .expect("Could not find the Arena Core's proposal module")
            .address,
    )?;

    // Check the result
    let result = arena.arena_wager_module.result(Uint128::one())?;
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().member_percentages[0].addr,
        user1.to_string()
    );

    Ok(())
}

#[test]
fn test_wager_with_stats() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;
    let user2 = mock.addr_make_with_balance("user2", coins(10000, DENOM))?;

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager
    let res = arena.arena_wager_module.create_competition(
        "A test wager with stats".to_string(),
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone(), user2.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
        "Test Wager with Stats".to_string(),
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
            })?,
            label: "Wager with Stats".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Wager Rule".to_string()]),
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

    // Add stat types
    arena.arena_wager_module.update_stat_types(
        Uint128::one(),
        vec![
            StatType {
                name: "wins".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(1),
                is_beneficial: true,
                aggregation_type: None,
            },
            StatType {
                name: "points".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(2),
                is_beneficial: true,
                aggregation_type: None,
            },
        ],
        vec![],
    )?;

    // Update stats
    arena.arena_wager_module.input_stats(
        Uint128::one(),
        vec![
            MemberStatsMsg {
                addr: user1.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "wins".to_string(),
                        value: StatValue::Uint(Uint128::one()),
                    },
                    StatMsg::InputStat {
                        name: "points".to_string(),
                        value: StatValue::Uint(Uint128::new(10)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: user2.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "wins".to_string(),
                        value: StatValue::Uint(Uint128::zero()),
                    },
                    StatMsg::InputStat {
                        name: "points".to_string(),
                        value: StatValue::Uint(Uint128::new(5)),
                    },
                ],
            },
        ],
    )?;

    // Check final stats
    let user1_stats = arena
        .arena_wager_module
        .historical_stats(user1.to_string(), Uint128::one())?;
    let user2_stats = arena
        .arena_wager_module
        .historical_stats(user2.to_string(), Uint128::one())?;

    assert_eq!(*user1_stats[0][1].value(), StatValue::Uint(Uint128::one())); // wins
    assert_eq!(
        *user1_stats[0][0].value(),
        StatValue::Uint(Uint128::new(10))
    ); // points
    assert_eq!(*user2_stats[0][1].value(), StatValue::Uint(Uint128::zero())); // wins
    assert_eq!(*user2_stats[0][0].value(), StatValue::Uint(Uint128::new(5))); // points

    Ok(())
}

#[test]
fn test_wager_with_aggregate_stats() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make_with_balance("user1", coins(10000, DENOM))?;

    arena.arena_wager_module.set_sender(&admin);

    // Create a wager
    let res = arena.arena_wager_module.create_competition(
        "A test wager with aggregate stats".to_string(),
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[user1.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        WagerInstantiateExt {},
        "Test Wager with Aggregate Stats".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: vec![MemberBalanceUnchecked {
                    addr: user1.to_string(),
                    balance: BalanceUnchecked {
                        native: Some(vec![Coin::new(1000, DENOM)]),
                        cw20: None,
                        cw721: None,
                    },
                }],
            })?,
            label: "Wager with Aggregate Stats".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Wager Rule".to_string()]),
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

    // Add stat types with aggregation
    arena.arena_wager_module.update_stat_types(
        Uint128::one(),
        vec![
            StatType {
                name: "total_wins".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(1),
                is_beneficial: true,
                aggregation_type: Some(StatAggregationType::Cumulative),
            },
            StatType {
                name: "average_score".to_string(),
                value_type: StatValueType::Decimal,
                tie_breaker_priority: Some(2),
                is_beneficial: true,
                aggregation_type: Some(StatAggregationType::Average),
            },
        ],
        vec![],
    )?;

    // Update stats multiple times
    for i in 0..3 {
        arena.arena_wager_module.input_stats(
            Uint128::one(),
            vec![MemberStatsMsg {
                addr: user1.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "total_wins".to_string(),
                        value: StatValue::Uint(Uint128::one()),
                    },
                    StatMsg::InputStat {
                        name: "average_score".to_string(),
                        value: StatValue::Decimal(Decimal::percent(20 * i)),
                    },
                ],
            }],
        )?;
        mock.next_block()?;
    }

    // Check historical stats
    let stats = arena
        .arena_wager_module
        .historical_stats(user1.to_string(), Uint128::one())?;

    assert_eq!(stats.len(), 3);

    // Check stats table with aggregation
    let stats_table = arena
        .arena_wager_module
        .stats_table(Uint128::one(), None, None)?;
    assert_eq!(stats_table.len(), 1);

    assert_eq!(
        *stats_table[0].stats[1].value(),
        StatValue::Uint(Uint128::new(3))
    );
    assert_eq!(
        *stats_table[0].stats[0].value(),
        StatValue::Decimal(Decimal::percent(20)),
    );
    mock.next_block()?;

    Ok(())
}

#[test]
#[ignore = "RPC blocks"]
fn test_migration_v2_v2_1() -> anyhow::Result<()> {
    let app = CloneTesting::new(PION_1)?;
    let mut arena = Arena::new(app.clone());
    const ARENA_DAO: &str = "neutron1ehkcl0n6s2jtdw75xsvfxm304mz4hs5z7jt6wn5mk0celpj0epqql4ulxk";
    let arena_dao_addr = Addr::unchecked(ARENA_DAO);

    arena.arena_group.upload()?;
    arena.arena_wager_module.upload()?;

    arena.arena_group.instantiate(
        &group::InstantiateMsg { members: None },
        Some(&arena_dao_addr),
        None,
    )?;

    arena.arena_wager_module.set_address(&Addr::unchecked(
        "neutron16nl0tcwt9qujavdakft7ddyw4pwzh5nuzn35tke9m4yfu462z99q6yj66n",
    ));
    arena.arena_wager_module.set_sender(&arena_dao_addr);

    arena.arena_wager_module.migrate(
        &MigrateMsg::WithGroupAddress {
            group_contract: arena.arena_group.addr_str()?,
        },
        arena.arena_wager_module.code_id()?,
    )?;

    Ok(())
}
