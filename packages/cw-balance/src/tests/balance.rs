use cosmwasm_std::{Addr, Coin, Uint128};
use cw20::Cw20CoinVerified;

use crate::{BalanceVerified, Cw721CollectionVerified};

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
fn test_difference_balances() {
    let balance_a = BalanceVerified {
        native: vec![
            Coin {
                denom: "token1".to_string(),
                amount: Uint128::from(100u64),
            },
            Coin {
                denom: "token2".to_string(),
                amount: Uint128::from(100u64),
            },
        ],
        cw20: vec![Cw20CoinVerified {
            address: Addr::unchecked("address1"),
            amount: Uint128::from(200u64),
        }],
        cw721: vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid1".to_string(), "tokenid2".to_string()],
        }],
    };
    let balance_b = BalanceVerified {
        native: vec![Coin {
            denom: "token1".to_string(),
            amount: Uint128::from(50u64),
        }],
        cw20: vec![Cw20CoinVerified {
            address: Addr::unchecked("address1"),
            amount: Uint128::from(100u64),
        }],
        cw721: vec![Cw721CollectionVerified {
            address: Addr::unchecked("address2"),
            token_ids: vec!["tokenid1".to_string()],
        }],
    };

    // Check a valid difference of balance b to balance a
    let diff_balance = balance_b.difference(&balance_a).unwrap();
    assert_eq!(diff_balance.native[0].amount, Uint128::from(50u64));
    assert_eq!(diff_balance.native[1].amount, Uint128::from(100u64));
    assert_eq!(diff_balance.cw20[0].amount, Uint128::from(100u64));
    assert_eq!(
        diff_balance.cw721[0].token_ids,
        vec!["tokenid2".to_string()]
    );

    // Assert no difference from balance a to balance b
    let diff_balance = balance_a.difference(&balance_b).unwrap();
    assert!(diff_balance.is_empty());
}
