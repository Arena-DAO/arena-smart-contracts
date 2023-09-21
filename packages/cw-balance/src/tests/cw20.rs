use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20CoinVerified;

use crate::BalanceVerified;

#[test]
fn test_add_cw20_balances() {
    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");

    // 10 + 10 = 20
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.cw20,
        vec![Cw20CoinVerified {
            address: addr1.clone(),
            amount: Uint128::from(20u128),
        }]
    );

    // 10 + Nothing = 10
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![];

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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.cw20,
        vec![Cw20CoinVerified {
            address: addr1.clone(),
            amount: Uint128::from(10u128),
        }]
    );

    // Nothing + 10 = 10
    let cw20_balance_a = vec![];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.cw20,
        vec![Cw20CoinVerified {
            address: addr1.clone(),
            amount: Uint128::from(10u128),
        }]
    );

    // addr1 10 + addr2 10 = addr1 10, addr2 10
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr2.clone(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert_eq!(
        new_balance.cw20,
        vec![
            Cw20CoinVerified {
                address: addr1.clone(),
                amount: Uint128::from(10u128),
            },
            Cw20CoinVerified {
                address: addr2.clone(),
                amount: Uint128::from(10u128),
            }
        ]
    );
}

#[test]
fn test_subtract_cw20_balances() {
    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");

    // 10 - 5 = 5
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(5u128),
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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(
        new_balance.cw20,
        vec![Cw20CoinVerified {
            address: addr1.clone(),
            amount: Uint128::from(5u128),
        }]
    );

    // 10 - 10 = Nothing
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert!(new_balance.is_empty());

    // 10 - Nothing = 10
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![];

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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert_eq!(
        new_balance.cw20,
        vec![Cw20CoinVerified {
            address: addr1.clone(),
            amount: Uint128::from(10u128),
        }]
    );

    // Nothing - 10 = Err
    let cw20_balance_a = vec![];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
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

    let new_balance_result = balance_a.checked_sub(&balance_b);
    assert!(new_balance_result.is_err());

    // 5 - 10 = Err
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(5u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
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

    let new_balance_result = balance_a.checked_sub(&balance_b);
    assert!(new_balance_result.is_err());

    // addr1 10 - addr2 10 = Err
    let cw20_balance_a = vec![Cw20CoinVerified {
        address: addr1.clone(),
        amount: Uint128::from(10u128),
    }];
    let cw20_balance_b = vec![Cw20CoinVerified {
        address: addr2.clone(),
        amount: Uint128::from(10u128),
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

    let new_balance_response = balance_a.checked_sub(&balance_b);
    assert!(new_balance_response.is_err())
}
