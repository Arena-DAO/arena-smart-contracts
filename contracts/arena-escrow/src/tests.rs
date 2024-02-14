use cosmwasm_std::{Addr, Binary, Coin, Empty, Uint128};
use cw20::{Cw20Coin, Cw20CoinVerified};
use cw_balance::{
    BalanceUnchecked, BalanceVerified, Cw721Collection, MemberBalanceUnchecked, MemberShare,
};
use cw_multi_test::{App, Executor};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

const CREATOR: &str = "creator";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";

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
                initial_balances: vec![
                    Cw20Coin {
                        address: ADDR1.to_string(),
                        amount: Uint128::from(1000u128),
                    },
                    Cw20Coin {
                        address: ADDR2.to_string(),
                        amount: Uint128::from(1000u128),
                    },
                ],
                mint: None,
                marketing: None,
            },
            &[],
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
            &[],
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
            &[],
        )
        .unwrap();
    }

    let escrow_addr = app
        .instantiate_contract(
            escrow_code_id,
            Addr::unchecked(CREATOR),
            &InstantiateMsg {
                dues: vec![
                    MemberBalanceUnchecked {
                        addr: ADDR1.to_string(),
                        balance: BalanceUnchecked {
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
                                address: cw721_addr.to_string(),
                                token_ids: vec![1.to_string()],
                            }],
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: ADDR2.to_string(),
                        balance: BalanceUnchecked {
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
                                address: cw721_addr.to_string(),
                                token_ids: vec![7.to_string()],
                            }],
                        },
                    },
                ],
            },
            &[],
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
            &[],
        )
        .unwrap();

    let res = context.app.execute_contract(
        Addr::unchecked(CREATOR),
        context.escrow_addr.clone(),
        &ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &[],
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
            &[],
        )
        .unwrap();

    let res = context.app.execute_contract(
        Addr::unchecked(CREATOR),
        context.escrow_addr.clone(),
        &ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &[],
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
        &[],
    );

    assert!(res.is_ok());

    let contract_distribution: Option<Vec<MemberShare<String>>> = context
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
                amount: Uint128::from(150u128),
                msg: Binary::default(),
            },
            &[],
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

    assert!(balance_addr1
        .difference(&BalanceVerified {
            native: vec![],
            cw20: vec![Cw20CoinVerified {
                address: context.cw20_addr.clone(),
                amount: Uint128::from(150u128),
            }],
            cw721: vec![],
        })
        .unwrap()
        .is_empty());
    let due_addr1: Option<BalanceVerified> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Due {
                addr: addr1.to_string(),
            },
        )
        .unwrap();
    assert!(due_addr1.is_some());

    let balance_total: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(context.escrow_addr.clone(), &QueryMsg::TotalBalance {})
        .unwrap();
    assert!(balance_total
        .difference(&BalanceVerified {
            native: vec![],
            cw20: vec![Cw20CoinVerified {
                address: context.cw20_addr.clone(),
                amount: Uint128::from(150u128),
            }],
            cw721: vec![],
        })
        .unwrap()
        .is_empty());

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
            &[],
        )
        .unwrap();

    // Check the updated balances
    let balance_addr1: Option<BalanceVerified> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Balance {
                addr: addr1.to_string(),
            },
        )
        .unwrap();
    let balance_total: Option<BalanceVerified> = context
        .app
        .wrap()
        .query_wasm_smart(context.escrow_addr.clone(), &QueryMsg::TotalBalance {})
        .unwrap();
    let due_addr1: BalanceVerified = context
        .app
        .wrap()
        .query_wasm_smart(
            context.escrow_addr.clone(),
            &QueryMsg::Due {
                addr: addr1.to_string(),
            },
        )
        .unwrap();
    assert!(due_addr1
        .difference(&BalanceVerified {
            native: vec![],
            cw20: vec![Cw20CoinVerified {
                address: context.cw20_addr.clone(),
                amount: Uint128::from(150u128),
            }],
            cw721: vec![],
        })
        .unwrap()
        .is_empty());
    assert!(balance_addr1.is_none());
    assert!(balance_total.is_none());
}
