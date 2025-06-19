use crate::contract::{instantiate, query};
use arena_interface::escrow::{InstantiateMsg, QueryMsg};
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{from_json, Coin, Uint128};
use cw20::Cw20Coin;
use cw_balance::{BalanceUnchecked, Cw721Collection, MemberBalanceUnchecked};

#[test]
fn test_instantiate_and_query_state() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let member1 = deps.api.addr_make("member1");
    let member2 = deps.api.addr_make("member2");
    let cw20_token = deps.api.addr_make("cw20_token");
    let cw721_contract = deps.api.addr_make("cw721_contract");
    let info = message_info(&creator, &[]);

    // Create test dues with native, CW20, and CW721 tokens
    let dues = vec![
        MemberBalanceUnchecked {
            addr: member1.to_string(),
            balance: BalanceUnchecked {
                native: vec![Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::new(1000),
                }],
                cw20: vec![Cw20Coin {
                    address: cw20_token.to_string(),
                    amount: Uint128::new(500),
                }],
                cw721: vec![Cw721Collection {
                    address: cw721_contract.to_string(),
                    token_ids: vec!["token1".to_string()],
                }],
            },
        },
        MemberBalanceUnchecked {
            addr: member2.to_string(),
            balance: BalanceUnchecked {
                native: vec![Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::new(2000),
                }],
                cw20: vec![],
                cw721: vec![],
            },
        },
    ];

    let msg = InstantiateMsg {
        dues,
        is_enrollment: false,
    };

    // Test instantiation
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Test IsLocked query - should be false for non-enrollment
    let query_msg = QueryMsg::IsLocked {};
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let is_locked: bool = from_json(&res).unwrap();
    assert!(!is_locked);

    // Test IsFullyFunded query - should be false since no funds were deposited
    let query_msg = QueryMsg::IsFullyFunded {};
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let is_fully_funded: bool = from_json(&res).unwrap();
    assert!(!is_fully_funded);

    // Test IsFunded query for member1 - should be false since no deposits made yet
    let query_msg = QueryMsg::IsFunded {
        addr: member1.to_string(),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let is_funded: bool = from_json(&res).unwrap();
    assert!(!is_funded);

    // Test Due query for member1
    let query_msg = QueryMsg::Due {
        addr: member1.to_string(),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let due_balance: cw_balance::BalanceVerified = from_json(&res).unwrap();

    // Check that dues were correctly stored
    assert_eq!(due_balance.native.get("uatom"), Some(&Uint128::new(1000)));
    assert_eq!(due_balance.cw20.get(&cw20_token), Some(&Uint128::new(500)));
    assert_eq!(due_balance.cw721.len(), 1);
    let nft_tokens = due_balance.cw721.get(&cw721_contract).unwrap();
    assert!(nft_tokens.contains("token1"));

    // Test Due query for member2
    let query_msg = QueryMsg::Due {
        addr: member2.to_string(),
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let due_balance: cw_balance::BalanceVerified = from_json(&res).unwrap();

    // Check member2's dues
    assert_eq!(due_balance.native.get("uatom"), Some(&Uint128::new(2000)));
    assert!(due_balance.cw20.is_empty());
    assert!(due_balance.cw721.is_empty());
}

#[test]
fn test_query_dues_pagination() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let creator = deps.api.addr_make("creator");
    let member1 = deps.api.addr_make("member1");
    let member2 = deps.api.addr_make("member2");
    let member3 = deps.api.addr_make("member3");
    let info = message_info(&creator, &[]);

    // Create multiple members with dues
    let dues = vec![
        MemberBalanceUnchecked {
            addr: member1.to_string(),
            balance: BalanceUnchecked {
                native: vec![Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::new(1000),
                }],
                cw20: vec![],
                cw721: vec![],
            },
        },
        MemberBalanceUnchecked {
            addr: member2.to_string(),
            balance: BalanceUnchecked {
                native: vec![Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::new(2000),
                }],
                cw20: vec![],
                cw721: vec![],
            },
        },
        MemberBalanceUnchecked {
            addr: member3.to_string(),
            balance: BalanceUnchecked {
                native: vec![Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::new(3000),
                }],
                cw20: vec![],
                cw721: vec![],
            },
        },
    ];

    let msg = InstantiateMsg {
        dues,
        is_enrollment: false,
    };

    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Test Dues query with pagination
    let query_msg = QueryMsg::Dues {
        start_after: None,
        limit: Some(2),
    };
    let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
    let dues_list: Vec<cw_balance::MemberBalanceChecked> = from_json(&res).unwrap();

    // Should return 2 members due to limit
    assert_eq!(dues_list.len(), 2);

    // Check that the dues are correct
    for member_due in dues_list {
        assert!(member_due.balance.native.contains_key("uatom"));
        let amount = member_due.balance.native.get("uatom").unwrap();
        assert!(amount > &Uint128::zero());
    }

    // Test InitialDues query
    let query_msg = QueryMsg::InitialDues {
        start_after: None,
        limit: None,
    };
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let initial_dues: Vec<cw_balance::MemberBalanceChecked> = from_json(&res).unwrap();

    // Should return all 3 members
    assert_eq!(initial_dues.len(), 3);
}
