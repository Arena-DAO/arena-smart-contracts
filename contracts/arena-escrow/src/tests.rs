use cosmwasm_std::{Addr, Coin, Decimal, StdError, Uint128};
use cw20::Cw20CoinVerified;
use cw_balance::{BalanceVerified, Cw721CollectionVerified, Distribution, MemberPercentage};

#[test]
fn test_add_empty_balances() {
    let balance_a = BalanceVerified::default();
    let balance_b = BalanceVerified::default();

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert!(new_balance.is_empty());
}

#[test]
fn test_subtract_empty_balances() {
    let balance_a = BalanceVerified::default();
    let balance_b = BalanceVerified::default();

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert!(new_balance.is_empty());
}

#[test]
fn test_checked_add_with_overflow() {
    let balance1 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::MAX,
        }]),
        cw20: None,
        cw721: None,
    };
    let balance2 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::new(1),
        }]),
        cw20: None,
        cw721: None,
    };

    let result = balance1.checked_add(&balance2);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StdError::Overflow { .. }));
}

#[test]
fn test_difference_balances() {
    let balance_a = BalanceVerified {
        native: Some(vec![
            Coin {
                denom: "token1".to_string(),
                amount: Uint128::new(100),
            },
            Coin {
                denom: "token2".to_string(),
                amount: Uint128::new(100),
            },
        ]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("address1"),
            amount: Uint128::new(200),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid1".to_string(), "tokenid2".to_string()],
        }]),
    };

    let balance_b = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token1".to_string(),
            amount: Uint128::new(50),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("address1"),
            amount: Uint128::new(100),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid1".to_string()],
        }]),
    };

    let diff_balance = balance_b.difference_to(&balance_a).unwrap();
    assert_eq!(
        diff_balance
            .native
            .unwrap()
            .iter()
            .find(|c| c.denom == "token2")
            .unwrap()
            .amount,
        Uint128::new(100)
    );
    assert_eq!(
        diff_balance
            .cw20
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("address1"))
            .unwrap()
            .amount,
        Uint128::new(100)
    );
    assert_eq!(
        diff_balance
            .cw721
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("address2"))
            .unwrap()
            .token_ids,
        vec!["tokenid2".to_string()]
    );

    let diff_balance = balance_a.difference_to(&balance_b).unwrap();
    assert!(diff_balance.is_empty());
}

#[test]
fn test_add_balances() {
    let balance_a = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token1".to_string(),
            amount: Uint128::new(100),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("address1"),
            amount: Uint128::new(200),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid1".to_string()],
        }]),
    };

    let balance_b = BalanceVerified {
        native: Some(vec![
            Coin {
                denom: "token1".to_string(),
                amount: Uint128::new(50),
            },
            Coin {
                denom: "token2".to_string(),
                amount: Uint128::new(75),
            },
        ]),
        cw20: Some(vec![
            Cw20CoinVerified {
                address: Addr::unchecked("address1"),
                amount: Uint128::new(100),
            },
            Cw20CoinVerified {
                address: Addr::unchecked("address3"),
                amount: Uint128::new(150),
            },
        ]),
        cw721: Some(vec![
            Cw721CollectionVerified {
                address: Addr::unchecked("address2"),
                token_ids: vec!["tokenid2".to_string()],
            },
            Cw721CollectionVerified {
                address: Addr::unchecked("address4"),
                token_ids: vec!["tokenid3".to_string()],
            },
        ]),
    };

    let new_balance = balance_a.checked_add(&balance_b).unwrap();

    assert_eq!(
        new_balance.native,
        Some(vec![
            Coin {
                denom: "token1".to_string(),
                amount: Uint128::new(150),
            },
            Coin {
                denom: "token2".to_string(),
                amount: Uint128::new(75),
            },
        ])
    );
    assert_eq!(
        new_balance.cw20,
        Some(vec![
            Cw20CoinVerified {
                address: Addr::unchecked("address1"),
                amount: Uint128::new(300),
            },
            Cw20CoinVerified {
                address: Addr::unchecked("address3"),
                amount: Uint128::new(150),
            },
        ])
    );
    assert_eq!(
        new_balance.cw721,
        Some(vec![
            Cw721CollectionVerified {
                address: Addr::unchecked("address2"),
                token_ids: vec!["tokenid1".to_string(), "tokenid2".to_string()],
            },
            Cw721CollectionVerified {
                address: Addr::unchecked("address4"),
                token_ids: vec!["tokenid3".to_string()],
            },
        ])
    );
}

#[test]
fn test_subtract_balances() {
    let balance_a = BalanceVerified {
        native: Some(vec![
            Coin {
                denom: "token1".to_string(),
                amount: Uint128::new(100),
            },
            Coin {
                denom: "token2".to_string(),
                amount: Uint128::new(75),
            },
        ]),
        cw20: Some(vec![
            Cw20CoinVerified {
                address: Addr::unchecked("address1"),
                amount: Uint128::new(200),
            },
            Cw20CoinVerified {
                address: Addr::unchecked("address3"),
                amount: Uint128::new(150),
            },
        ]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid1".to_string(), "tokenid2".to_string()],
        }]),
    };

    let balance_b = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token1".to_string(),
            amount: Uint128::new(50),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("address1"),
            amount: Uint128::new(100),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid1".to_string()],
        }]),
    };

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();

    assert_eq!(
        new_balance.native,
        Some(vec![
            Coin {
                denom: "token1".to_string(),
                amount: Uint128::new(50),
            },
            Coin {
                denom: "token2".to_string(),
                amount: Uint128::new(75),
            },
        ])
    );
    assert_eq!(
        new_balance.cw20,
        Some(vec![
            Cw20CoinVerified {
                address: Addr::unchecked("address1"),
                amount: Uint128::new(100),
            },
            Cw20CoinVerified {
                address: Addr::unchecked("address3"),
                amount: Uint128::new(150),
            },
        ])
    );
    assert_eq!(
        new_balance.cw721,
        Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid2".to_string()],
        }])
    );
}

#[test]
fn test_checked_sub_with_insufficient_balance() {
    let balance1 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::new(50),
        }]),
        cw20: None,
        cw721: None,
    };
    let balance2 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::new(100),
        }]),
        cw20: None,
        cw721: None,
    };

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
}

#[test]
fn test_checked_mul_with_zero() {
    let balance = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::new(100),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: Uint128::new(200),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("cw721"),
            token_ids: vec!["1".to_string(), "2".to_string()],
        }]),
    };

    let result = balance.checked_mul_floor(Decimal::zero()).unwrap();
    assert!(result.native.is_none());
    assert!(result.cw20.is_none());
    assert!(result.cw721.is_none());
}

#[test]
fn test_split_with_invalid_distribution() {
    let balance = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::new(100),
        }]),
        cw20: None,
        cw721: None,
    };

    let distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: Addr::unchecked("addr1"),
                percentage: Decimal::percent(60),
            },
            MemberPercentage {
                addr: Addr::unchecked("addr2"),
                percentage: Decimal::percent(60),
            },
        ],
        remainder_addr: Addr::unchecked("remainder"),
    };

    let result = balance.split(&distribution);
    assert!(result.is_err());
}

#[test]
fn test_difference_with_empty_balances() {
    let balance1 = BalanceVerified::default();
    let balance2 = BalanceVerified::default();

    let result = balance1.difference_to(&balance2).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_checked_add_with_different_token_types() {
    let balance1 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token1".to_string(),
            amount: Uint128::new(100),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20_1"),
            amount: Uint128::new(200),
        }]),
        cw721: None,
    };
    let balance2 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token2".to_string(),
            amount: Uint128::new(50),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20_2"),
            amount: Uint128::new(150),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("cw721"),
            token_ids: vec!["1".to_string()],
        }]),
    };

    let result = balance1.checked_add(&balance2).unwrap();
    assert_eq!(result.native.unwrap().len(), 2);
    assert_eq!(result.cw20.unwrap().len(), 2);
    assert_eq!(result.cw721.unwrap().len(), 1);
}

#[test]
fn test_checked_sub_with_partial_amounts() {
    let balance1 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::new(100),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: Uint128::new(200),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("cw721"),
            token_ids: vec!["1".to_string(), "2".to_string()],
        }]),
    };
    let balance2 = BalanceVerified {
        native: Some(vec![Coin {
            denom: "token".to_string(),
            amount: Uint128::new(60),
        }]),
        cw20: Some(vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20"),
            amount: Uint128::new(50),
        }]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("cw721"),
            token_ids: vec!["1".to_string()],
        }]),
    };

    let result = balance1.checked_sub(&balance2).unwrap();
    assert_eq!(
        result
            .native
            .unwrap()
            .iter()
            .find(|c| c.denom == "token")
            .unwrap()
            .amount,
        Uint128::new(40)
    );
    assert_eq!(
        result
            .cw20
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("cw20"))
            .unwrap()
            .amount,
        Uint128::new(150)
    );
    assert_eq!(
        result
            .cw721
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("cw721"))
            .unwrap()
            .token_ids,
        vec!["2".to_string()]
    );
}
