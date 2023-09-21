use cosmwasm_std::{Coin, Uint128};

use crate::BalanceVerified;

#[test]
fn test_add_native_balances() {
    let denom1 = "denom1";
    let denom2 = "denom2";

    // 10 + 10 = 20
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.native,
        vec![Coin {
            denom: denom1.to_string(),
            amount: Uint128::from(20u128)
        }]
    );

    // 10 + Nothing = 10
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![];

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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.native,
        vec![Coin {
            denom: denom1.to_string(),
            amount: Uint128::from(10u128)
        }]
    );

    // Nothing + 10 = 10
    let native_balance_a = vec![];
    let native_balance_b = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.native,
        vec![Coin {
            denom: denom1.to_string(),
            amount: Uint128::from(10u128)
        }]
    );

    // token1 10 + token2 10 = token1 10, token2 10
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: denom2.to_string(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.native,
        vec![
            Coin {
                denom: denom1.to_string(),
                amount: Uint128::from(10u128)
            },
            Coin {
                denom: denom2.to_string(),
                amount: Uint128::from(10u128),
            }
        ]
    );
}

#[test]
fn test_subtract_native_balances() {
    let denom1 = "denom1";
    let denom2 = "denom2";

    // 10 - 5 = 5
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(5u128),
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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(
        new_balance.native,
        vec![Coin {
            denom: denom1.to_string(),
            amount: Uint128::from(5u128),
        }]
    );

    // 10 - 10 = Nothing
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert!(new_balance.is_empty());

    // 10 - Nothing = 10
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![];

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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(
        new_balance.native,
        vec![Coin {
            denom: denom1.to_string(),
            amount: Uint128::from(10u128),
        }]
    );

    // Nothing - 10 = Err
    let native_balance_a = vec![];
    let native_balance_b = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
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

    let new_balance_result = balance_a.checked_sub(&balance_b);
    assert!(new_balance_result.is_err());

    // 5 - 10 = Err
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(5u128),
    }];
    let native_balance_b = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
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

    let new_balance_result = balance_a.checked_sub(&balance_b);
    assert!(new_balance_result.is_err());

    // token1 10 - token2 10 = Err
    let native_balance_a = vec![Coin {
        denom: denom1.to_string(),
        amount: Uint128::from(10u128),
    }];
    let native_balance_b = vec![Coin {
        denom: denom2.to_string(),
        amount: Uint128::from(10u128),
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

    let new_balance_response = balance_a.checked_sub(&balance_b);
    assert!(new_balance_response.is_err())
}
