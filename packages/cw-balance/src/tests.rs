use std::collections::HashSet;

use cosmwasm_std::{Addr, Coin, Uint128};
use cw20::Cw20CoinVerified;

use crate::{balance::BalanceVerified, Cw721CollectionVerified, MemberShareVerified};

#[test]
fn test_add_native_balances() {
    let native_balance_a = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(20u128),
    }];

    let balance_a = BalanceVerified {
        native: native_balance_a,
        cw20: vec![],
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: native_balance_b,
        cw20: vec![],
        cw721: vec![],
    };

    let combined_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        combined_balance.native,
        vec![Coin {
            denom: "token1".to_string(),
            amount: Uint128::from(30u128)
        }]
    );
}

#[test]
fn test_add_cw20_balances() {
    let addr = Addr::unchecked("cw20token");
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr.clone(),
        amount: Uint128::from(20u128),
    }];

    let balance_a = BalanceVerified {
        native: vec![],
        cw20: cw20_balance_a,
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: vec![],
        cw20: cw20_balance_b,
        cw721: vec![],
    };

    let combined_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        combined_balance.cw20,
        vec![Cw20CoinVerified {
            address: addr.clone(),
            amount: Uint128::from(30u128)
        }]
    );
}

#[test]
fn test_add_cw721_balances() {
    let addr = Addr::unchecked("cw721token");
    let cw721_balance_a = vec![Cw721CollectionVerified {
        addr: addr.clone(),
        token_ids: vec!["token1".to_string(), "token2".to_string()],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        addr: addr.clone(),
        token_ids: vec!["token3".to_string(), "token4".to_string()],
    }];

    let balance_a = BalanceVerified {
        native: vec![],
        cw20: vec![],
        cw721: cw721_balance_a,
    };
    let balance_b = BalanceVerified {
        native: vec![],
        cw20: vec![],
        cw721: cw721_balance_b,
    };

    let combined_balance = balance_a.checked_add(&balance_b).unwrap();
    let combined_cw721_tokens = combined_balance
        .cw721
        .iter()
        .find(|tokens| tokens.addr == addr)
        .unwrap();
    assert_eq!(combined_cw721_tokens.token_ids.len(), 4);
    assert!(combined_cw721_tokens
        .token_ids
        .contains(&"token1".to_string()));
    assert!(combined_cw721_tokens
        .token_ids
        .contains(&"token2".to_string()));
    assert!(combined_cw721_tokens
        .token_ids
        .contains(&"token3".to_string()));
    assert!(combined_cw721_tokens
        .token_ids
        .contains(&"token4".to_string()));
}

#[test]
fn test_subtract_native_balances() {
    let native_balance_a = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(30u128),
    }];
    let native_balance_b = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(20u128),
    }];

    let balance_a = BalanceVerified {
        native: native_balance_a,
        cw20: vec![],
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: native_balance_b,
        cw20: vec![],
        cw721: vec![],
    };

    let remaining_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(
        remaining_balance.native,
        vec![Coin {
            denom: "token1".to_string(),
            amount: Uint128::from(10u128),
        }]
    );
}

#[test]
fn test_subtract_native_balances_empty() {
    let native_balance_a = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(30u128),
    }];
    let native_balance_b = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(30u128),
    }];

    let balance_a = BalanceVerified {
        native: native_balance_a,
        cw20: vec![],
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: native_balance_b,
        cw20: vec![],
        cw721: vec![],
    };

    let remaining_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(remaining_balance.native, vec![]);
}

#[test]
fn test_subtract_cw20_balances() {
    let addr = Addr::unchecked("cw20token");
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr.clone(),
        amount: Uint128::from(30u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr.clone(),
        amount: Uint128::from(20u128),
    }];

    let balance_a = BalanceVerified {
        native: vec![],
        cw20: cw20_balance_a,
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: vec![],
        cw20: cw20_balance_b,
        cw721: vec![],
    };

    let remaining_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(
        remaining_balance.cw20,
        vec![Cw20CoinVerified {
            address: addr.clone(),
            amount: Uint128::from(10u128),
        }]
    );
}

#[test]
fn test_subtract_cw721_balances() {
    let addr = Addr::unchecked("cw721token");
    let cw721_balance_a = vec![Cw721CollectionVerified {
        addr: addr.clone(),
        token_ids: vec![
            "token1".to_string(),
            "token2".to_string(),
            "token3".to_string(),
            "token4".to_string(),
        ],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        addr: addr.clone(),
        token_ids: vec!["token1".to_string(), "token2".to_string()],
    }];

    let balance_a = BalanceVerified {
        native: vec![],
        cw20: vec![],
        cw721: cw721_balance_a,
    };
    let balance_b = BalanceVerified {
        native: vec![],
        cw20: vec![],
        cw721: cw721_balance_b,
    };

    let remaining_balance = balance_a.checked_sub(&balance_b).unwrap();

    // Convert the result's token_ids into a HashSet
    let result_tokens: HashSet<_> = remaining_balance.cw721[0]
        .token_ids
        .iter()
        .cloned()
        .collect();

    // Assert the token_ids are equivalent
    assert_eq!(
        result_tokens,
        ["token3", "token4"].iter().map(|s| s.to_string()).collect()
    );
}

#[test]
fn test_add_different_native_denoms() {
    let native_balance_a = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: "token2".to_string(),
        amount: Uint128::from(20u128),
    }];

    let balance_a = BalanceVerified {
        native: native_balance_a,
        cw20: vec![],
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: native_balance_b,
        cw20: vec![],
        cw721: vec![],
    };

    let combined_balance = balance_a.checked_add(&balance_b).unwrap();

    assert_eq!(
        combined_balance.get_amount(crate::TokenType::Native, "token1"),
        Some(Uint128::from(10u128))
    );
    assert_eq!(
        combined_balance.get_amount(crate::TokenType::Native, "token2"),
        Some(Uint128::from(20u128))
    );
}

#[test]
fn test_subtract_insufficient_native_balance() {
    let native_balance_a = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: "token1".to_string(),
        amount: Uint128::from(20u128),
    }];

    let balance_a = BalanceVerified {
        native: native_balance_a,
        cw20: vec![],
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: native_balance_b,
        cw20: vec![],
        cw721: vec![],
    };

    assert!(balance_a.checked_sub(&balance_b).is_err());
}

#[test]
fn test_add_empty_balances() {
    let balance_a = BalanceVerified::default();
    let balance_b = BalanceVerified::default();

    let combined_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(combined_balance.native.len(), 0);
    assert_eq!(combined_balance.cw20.len(), 0);
    assert_eq!(combined_balance.cw721.len(), 0);
}

#[test]
fn test_subtract_empty_balances() {
    let balance_a = BalanceVerified::default();
    let balance_b = BalanceVerified::default();

    let remaining_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(remaining_balance.native.len(), 0);
    assert_eq!(remaining_balance.cw20.len(), 0);
    assert_eq!(remaining_balance.cw721.len(), 0);
}

#[test]
fn test_add_multiple_cw20_contract_addresses() {
    let addr1 = Addr::unchecked("cw20token1");
    let addr2 = Addr::unchecked("cw20token2");
    let cw20_balance_a = vec![
        Cw20CoinVerified {
            address: addr1.clone(),
            amount: Uint128::from(10u128),
        },
        Cw20CoinVerified {
            address: addr2.clone(),
            amount: Uint128::from(20u128),
        },
    ];
    let cw20_balance_b = vec![
        Cw20CoinVerified {
            address: addr1.clone(),
            amount: Uint128::from(5u128),
        },
        Cw20CoinVerified {
            address: addr2.clone(),
            amount: Uint128::from(15u128),
        },
    ];

    let balance_a = BalanceVerified {
        native: vec![],
        cw20: cw20_balance_a,
        cw721: vec![],
    };
    let balance_b = BalanceVerified {
        native: vec![],
        cw20: cw20_balance_b,
        cw721: vec![],
    };

    let combined_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(combined_balance.cw20.len(), 2);
    assert_eq!(
        combined_balance
            .cw20
            .iter()
            .find(|coin| coin.address == addr1)
            .unwrap()
            .amount,
        Uint128::from(15u128)
    );
    assert_eq!(
        combined_balance
            .cw20
            .iter()
            .find(|coin| coin.address == addr2)
            .unwrap()
            .amount,
        Uint128::from(35u128)
    );
}

#[test]
fn test_subtract_nonexistent_cw721_tokens() {
    let addr = Addr::unchecked("cw721token");

    let cw721_balance_a = vec![Cw721CollectionVerified {
        addr: addr.clone(),
        token_ids: vec!["token1".to_string(), "token2".to_string()],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        addr: addr.clone(),
        token_ids: vec!["token3".to_string(), "token4".to_string()],
    }];

    let balance_a = BalanceVerified {
        native: vec![],
        cw20: vec![],
        cw721: cw721_balance_a,
    };
    let balance_b = BalanceVerified {
        native: vec![],
        cw20: vec![],
        cw721: cw721_balance_b,
    };

    let result = balance_a.checked_sub(&balance_b);
    assert!(result.is_err());
}

#[test]
fn test_split_balances() {
    let addr_a = Addr::unchecked("addr_a");
    let addr_b = Addr::unchecked("addr_b");
    let addr_c = Addr::unchecked("addr_c");

    let balance = BalanceVerified {
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
        cw20: vec![
            Cw20CoinVerified {
                address: Addr::unchecked("cw20token1"),
                amount: Uint128::from(150u128),
            },
            Cw20CoinVerified {
                address: Addr::unchecked("cw20token2"),
                amount: Uint128::from(50u128),
            },
        ],
        cw721: vec![Cw721CollectionVerified {
            addr: Addr::unchecked("cw721token1"),
            token_ids: vec!["1".to_string(), "2".to_string()],
        }],
    };

    let distribution = vec![
        MemberShareVerified {
            addr: addr_a.clone(),
            shares: Uint128::new(50u128),
        },
        MemberShareVerified {
            addr: addr_b.clone(),
            shares: Uint128::new(30u128),
        },
    ];

    let remainder_address = &addr_c;

    let split_result = balance.split(&distribution, remainder_address).unwrap();
    assert_eq!(split_result.len(), 3);

    let member_a_balance = split_result
        .iter()
        .find(|mb| mb.addr == addr_a)
        .unwrap()
        .balance
        .clone();
    let member_b_balance = split_result
        .iter()
        .find(|mb| mb.addr == addr_b)
        .unwrap()
        .balance
        .clone();
    let remainder_balance = split_result
        .iter()
        .find(|mb| mb.addr == addr_c)
        .unwrap()
        .balance
        .clone();

    // Check split amounts
    let member_a_native1 = member_a_balance
        .native
        .iter()
        .find(|coin| coin.denom == "native1")
        .unwrap();
    let member_a_native2 = member_a_balance
        .native
        .iter()
        .find(|coin| coin.denom == "native2")
        .unwrap();
    assert_eq!(member_a_native1.amount, Uint128::from(125u128));
    assert_eq!(member_a_native2.amount, Uint128::from(62u128));

    let member_a_cw20token1 = member_a_balance
        .cw20
        .iter()
        .find(|coin| coin.address == Addr::unchecked("cw20token1"))
        .unwrap();
    let member_a_cw20token2 = member_a_balance
        .cw20
        .iter()
        .find(|coin| coin.address == Addr::unchecked("cw20token2"))
        .unwrap();
    assert_eq!(member_a_cw20token1.amount, Uint128::from(93u128));
    assert_eq!(member_a_cw20token2.amount, Uint128::from(31u128));

    let member_b_native1 = member_b_balance
        .native
        .iter()
        .find(|coin| coin.denom == "native1")
        .unwrap();
    let member_b_native2 = member_b_balance
        .native
        .iter()
        .find(|coin| coin.denom == "native2")
        .unwrap();
    assert_eq!(member_b_native1.amount, Uint128::from(75u128));
    assert_eq!(member_b_native2.amount, Uint128::from(37u128));

    let member_b_cw20token1 = member_b_balance
        .cw20
        .iter()
        .find(|coin| coin.address == Addr::unchecked("cw20token1"))
        .unwrap();
    let member_b_cw20token2 = member_b_balance
        .cw20
        .iter()
        .find(|coin| coin.address == Addr::unchecked("cw20token2"))
        .unwrap();
    assert_eq!(member_b_cw20token1.amount, Uint128::from(56u128));
    assert_eq!(member_b_cw20token2.amount, Uint128::from(18u128));

    let remainder_native1 = remainder_balance
        .native
        .iter()
        .find(|coin| coin.denom == "native1")
        .unwrap();
    let remainder_native2 = remainder_balance
        .native
        .iter()
        .find(|coin| coin.denom == "native2")
        .unwrap();
    assert_eq!(remainder_native1.amount, Uint128::from(0u128));
    assert_eq!(remainder_native2.amount, Uint128::from(1u128));

    let remainder_cw20token1 = remainder_balance
        .cw20
        .iter()
        .find(|coin| coin.address == Addr::unchecked("cw20token1"))
        .unwrap();
    let remainder_cw20token2 = remainder_balance
        .cw20
        .iter()
        .find(|coin| coin.address == Addr::unchecked("cw20token2"))
        .unwrap();
    let remainder_cw721token1 = remainder_balance
        .cw721
        .iter()
        .find(|coin| coin.addr == Addr::unchecked("cw721token1"))
        .unwrap();
    assert_eq!(remainder_cw20token1.amount, Uint128::from(1u128));
    assert_eq!(remainder_cw20token2.amount, Uint128::from(1u128));
    assert_eq!(remainder_cw721token1.token_ids, vec!["1", "2"]);
}
