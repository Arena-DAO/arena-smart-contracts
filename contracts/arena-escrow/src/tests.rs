use cosmwasm_std::{Addr, Binary, Coin, Empty, Uint128};
use cw20::Cw20Coin;
use cw_balance::{Balance, BalanceVerified, Cw721Collection, MemberBalance, MemberShare};
use cw_competition::escrow::CompetitionEscrowDistributeMsg;
use cw_multi_test::{App, Executor};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

const CREATOR: &str = "creator";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const ADDR3: &str = "addr3";
const REMAINDER: &str = "remainder";

struct Context {
    pub app: App,
    pub escrow_addr: Addr,
    pub cw20_addr: Addr,
}

fn setup() -> Context {
    let mut app = App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(ADDR1),
                vec![
                    Coin {
                        denom: "native1".to_string(),
                        amount: Uint128::from(1000u128),
                    },
                    Coin {
                        denom: "native2".to_string(),
                        amount: Uint128::from(1000u128),
                    },
                ],
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(ADDR2),
                vec![
                    Coin {
                        denom: "native1".to_string(),
                        amount: Uint128::from(1000u128),
                    },
                    Coin {
                        denom: "native2".to_string(),
                        amount: Uint128::from(1000u128),
                    },
                ],
            )
            .unwrap();
    });
    let escrow_code_id = app.store_code(arena_testing::contracts::arena_dao_escrow_contract());
    let cw20_code_id = app.store_code(arena_testing::contracts::cw20_base_contract());
    let cw721_code_id = app.store_code(arena_testing::contracts::cw721_base_contract());

    // Instantiate the CW20 token contract
    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked(CREATOR),
            &cw20_base::msg::InstantiateMsg {
                name: "TestToken".to_string(),
                symbol: "TEST".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: ADDR1.to_string(),
                    amount: Uint128::from(1000u128),
                }],
                mint: None,
                marketing: None,
            },
            &vec![],
            "TestToken",
            None,
        )
        .unwrap();

    let cw721_addr = app
        .instantiate_contract(
            cw721_code_id,
            Addr::unchecked(CREATOR),
            &cw721_base::msg::InstantiateMsg {
                name: "TestNFTCollection".to_string(),
                symbol: "TESTNFT".to_string(),
                minter: CREATOR.to_string(),
            },
            &vec![],
            "TestToken",
            None,
        )
        .unwrap();

    for i in 1..12 {
        app.execute_contract(
            Addr::unchecked(CREATOR),
            cw721_addr.clone(),
            &cw721_base::ExecuteMsg::<Option<Empty>, Empty>::Mint {
                token_id: i.to_string(),
                owner: match i <= 6 {
                    true => ADDR1,
                    false => ADDR2,
                }
                .to_string(),
                token_uri: None,
                extension: None,
            },
            &vec![],
        )
        .unwrap();
    }

    let escrow_addr = app
        .instantiate_contract(
            escrow_code_id,
            Addr::unchecked(CREATOR),
            &InstantiateMsg {
                dues: vec![
                    MemberBalance {
                        addr: ADDR1.to_string(),
                        balance: Balance {
                            native: vec![
                                Coin {
                                    denom: "native1".to_string(),
                                    amount: Uint128::from(100u128),
                                },
                                Coin {
                                    denom: "native2".to_string(),
                                    amount: Uint128::from(50u128),
                                },
                            ],
                            cw20: vec![Cw20Coin {
                                address: cw20_addr.to_string(),
                                amount: Uint128::from(150u128),
                            }],
                            cw721: vec![Cw721Collection {
                                addr: cw721_addr.to_string(),
                                token_ids: vec![1.to_string(), 2.to_string(), 3.to_string()],
                            }],
                        },
                    },
                    MemberBalance {
                        addr: ADDR2.to_string(),
                        balance: Balance {
                            native: vec![
                                Coin {
                                    denom: "native1".to_string(),
                                    amount: Uint128::from(200u128),
                                },
                                Coin {
                                    denom: "native2".to_string(),
                                    amount: Uint128::from(100u128),
                                },
                            ],
                            cw20: vec![Cw20Coin {
                                address: cw20_addr.to_string(),
                                amount: Uint128::from(300u128),
                            }],
                            cw721: vec![Cw721Collection {
                                addr: cw721_addr.to_string(),
                                token_ids: vec![7.to_string(), 8.to_string(), 9.to_string()],
                            }],
                        },
                    },
                ],
            },
            &vec![],
            "Arena Escrow",
            None,
        )
        .unwrap();

    Context {
        app,
        escrow_addr,
        cw20_addr,
    }
}

#[test]
fn test_lock() {
    let mut context = setup();

    // Try to withdraw when the contract is locked
    context
        .app
        .execute_contract(
            Addr::unchecked(CREATOR),
            context.escrow_addr.clone(),
            &ExecuteMsg::Lock { value: true },
            &vec![],
        )
        .unwrap();

    let res = context.app.execute_contract(
        Addr::unchecked(CREATOR),
        context.escrow_addr.clone(),
        &ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &vec![],
    );
    assert_eq!(
        res.unwrap_err().root_cause().to_string(),
        ContractError::Locked {}.to_string()
    );

    // Try to withdraw when the contract is unlocked
    context
        .app
        .execute_contract(
            Addr::unchecked(CREATOR),
            context.escrow_addr.clone(),
            &ExecuteMsg::Lock { value: false },
            &vec![],
        )
        .unwrap();

    let res = context.app.execute_contract(
        Addr::unchecked(CREATOR),
        context.escrow_addr.clone(),
        &ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &vec![],
    );
    assert!(res.is_ok());
}

#[test]
fn test_set_distribution() {
    let mut context = setup();

    let distribution = vec![
        MemberShare {
            addr: ADDR1.to_string(),
            shares: Uint128::new(50),
        },
        MemberShare {
            addr: ADDR2.to_string(),
            shares: Uint128::new(30),
        },
    ];

    let res = context.app.execute_contract(
        Addr::unchecked(ADDR1),
        context.escrow_addr.clone(),
        &ExecuteMsg::SetDistribution {
            distribution: distribution.clone(),
        },
        &vec![],
    );

    assert!(res.is_ok());

    let contract_distribution: Option<Vec<MemberShare>> = context
        .app
        .wrap()
        .query_wasm_smart(
            &context.escrow_addr,
            &QueryMsg::Distribution {
                addr: ADDR1.to_string(),
            },
        )
        .unwrap();

    assert_eq!(contract_distribution, Some(distribution));
}

#[test]
fn test_deposit_withdraw_and_check_balances() {
    let mut context = setup();

    // Member addresses
    let addr1 = Addr::unchecked(ADDR1.to_string());

    // Deposit cw20 tokens to the contract
    context
        .app
        .execute_contract(
            addr1.clone(),
            context.cw20_addr.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: context.escrow_addr.to_string(),
                amount: Uint128::from(100u128),
                msg: Binary::default(),
            },
            &vec![],
        )
        .unwrap();

    // Check the updated balances
    let balance_addr1: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Balance {
                addr: addr1.to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        balance_addr1.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(100u128)
    );
    let balance_total: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(context.escrow_addr.clone(), &QueryMsg::TotalBalance {})
        .unwrap();
    assert_eq!(
        balance_total.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(100u128)
    );

    // Withdraw
    context
        .app
        .execute_contract(
            addr1.clone(),
            context.escrow_addr.clone(),
            &ExecuteMsg::Withdraw {
                cw20_msg: None,
                cw721_msg: None,
            },
            &vec![],
        )
        .unwrap();

    // Check the updated balances
    let balance_addr1: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Balance {
                addr: addr1.to_string(),
            },
        )
        .unwrap();
    let balance_total: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(context.escrow_addr.clone(), &QueryMsg::TotalBalance {})
        .unwrap();

    assert_eq!(
        balance_addr1.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(0u128)
    );
    assert_eq!(
        balance_total.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(0u128)
    );
}

#[test]
fn test_distribute_without_preset_distribution() {
    let mut context = setup();

    // Member addresses
    let creator = Addr::unchecked(CREATOR.to_string());
    let addr1 = Addr::unchecked(ADDR1.to_string());
    let addr2 = Addr::unchecked(ADDR2.to_string());
    let addr3 = Addr::unchecked(ADDR3.to_string());
    let remainder = Addr::unchecked(REMAINDER.to_string());

    // Fund the escrow
    context
        .app
        .execute_contract(
            addr1.clone(),
            context.cw20_addr.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: context.escrow_addr.to_string(),
                amount: Uint128::from(1000u128),
                msg: Binary::default(),
            },
            &vec![],
        )
        .unwrap();

    // Set up the distribution.
    let distribution = vec![
        MemberShare {
            addr: addr1.to_string(),
            shares: Uint128::from(1u128),
        },
        MemberShare {
            addr: addr2.to_string(),
            shares: Uint128::from(1u128),
        },
        MemberShare {
            addr: addr3.to_string(),
            shares: Uint128::from(1u128),
        },
    ];

    // Call the distribute function.
    context
        .app
        .execute_contract(
            creator.clone(),
            context.escrow_addr.clone(),
            &ExecuteMsg::Distribute(CompetitionEscrowDistributeMsg {
                distribution: Some(distribution),
                remainder_addr: remainder.to_string(),
            }),
            &vec![],
        )
        .unwrap();

    // Check the balances.
    let balance_addr1: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Balance {
                addr: addr1.to_string(),
            },
        )
        .unwrap();
    let balance_addr2: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Balance {
                addr: addr2.to_string(),
            },
        )
        .unwrap();
    let balance_addr3: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Balance {
                addr: addr3.to_string(),
            },
        )
        .unwrap();
    let balance_remainder: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Balance {
                addr: remainder.to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        balance_addr1.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(333u128)
    );
    assert_eq!(
        balance_addr2.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(333u128)
    );
    assert_eq!(
        balance_addr3.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(333u128)
    );
    assert_eq!(
        balance_remainder.get_amount(cw_balance::TokenType::Cw20, &context.cw20_addr.to_string()),
        Uint128::from(1u128)
    );
}
