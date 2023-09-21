use cosmwasm_std::Addr;

use crate::{BalanceVerified, Cw721CollectionVerified};

#[test]
fn test_add_cw721_balances() {
    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");

    // addr1token1 + addr1token2 = addr1token1token2
    let cw721_balance_a = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token2".to_string()],
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();

    assert_eq!(
        new_balance.cw721,
        vec![Cw721CollectionVerified {
            address: addr1.clone(),
            token_ids: vec!["token1".to_string(), "token2".to_string()]
        }]
    );

    // addr1token1 + addr2token1 = addr1token1 addr2token1
    let cw721_balance_a = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        address: addr2.clone(),
        token_ids: vec!["token1".to_string()],
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

    let new_balance = balance_a.checked_add(&balance_b).unwrap();

    assert_eq!(
        new_balance.cw721,
        vec![
            Cw721CollectionVerified {
                address: addr1.clone(),
                token_ids: vec!["token1".to_string()]
            },
            Cw721CollectionVerified {
                address: addr2.clone(),
                token_ids: vec!["token1".to_string()]
            }
        ]
    );

    // addr1token1 + addr1token1 = Err
    let cw721_balance_a = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
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

    let new_balance_response = balance_a.checked_add(&balance_b);

    assert!(new_balance_response.is_err());
}

#[test]
fn test_subtract_cw721_balances() {
    let addr1 = Addr::unchecked("addr1");
    let addr2 = Addr::unchecked("addr2");

    // addr1token1 - addr1token1 = Nothing
    let cw721_balance_a = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();

    assert!(new_balance.is_empty());

    // addr1token1 - addr2token1 = Err
    let cw721_balance_a = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
    }];
    let cw721_balance_b = vec![Cw721CollectionVerified {
        address: addr2.clone(),
        token_ids: vec!["token1".to_string()],
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

    let new_balance_response = balance_a.checked_sub(&balance_b);

    assert!(new_balance_response.is_err());

    // addr1token1 - Nothing = addr1token1
    let cw721_balance_a = vec![Cw721CollectionVerified {
        address: addr1.clone(),
        token_ids: vec!["token1".to_string()],
    }];
    let cw721_balance_b = vec![];

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

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();

    assert_eq!(
        new_balance.cw721,
        vec![Cw721CollectionVerified {
            address: addr1.clone(),
            token_ids: vec!["token1".to_string()],
        }]
    );
}
