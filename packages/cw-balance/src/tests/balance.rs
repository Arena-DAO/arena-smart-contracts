use crate::{BalanceError, BalanceVerified, Distribution, MemberPercentage};
use cosmwasm_std::{Addr, Decimal, Uint128};
use std::collections::{BTreeMap, BTreeSet};

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
        native: Some(BTreeMap::from([("token".to_string(), Uint128::MAX)])),
        cw20: None,
        cw721: None,
    };
    let balance2 = BalanceVerified {
        native: Some(BTreeMap::from([("token".to_string(), Uint128::new(1))])),
        cw20: None,
        cw721: None,
    };

    let result = balance1.checked_add(&balance2);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BalanceError::OverflowError(_)
    ));
}

#[test]
fn test_difference_balances() {
    let balance_a = BalanceVerified {
        native: Some(BTreeMap::from([
            ("token1".to_string(), Uint128::new(100)),
            ("token2".to_string(), Uint128::new(100)),
        ])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("address1"),
            Uint128::new(200),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("address2"),
            BTreeSet::from(["tokenid1".to_string(), "tokenid2".to_string()]),
        )])),
    };

    let balance_b = BalanceVerified {
        native: Some(BTreeMap::from([("token1".to_string(), Uint128::new(50))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("address1"),
            Uint128::new(100),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("address2"),
            BTreeSet::from(["tokenid1".to_string()]),
        )])),
    };

    let diff_balance = balance_b.difference(&balance_a).unwrap();
    assert_eq!(
        diff_balance.native.unwrap().get("token2"),
        Some(&Uint128::new(100))
    );
    assert_eq!(
        diff_balance.cw20.unwrap().get(&Addr::unchecked("address1")),
        Some(&Uint128::new(100))
    );
    assert_eq!(
        diff_balance
            .cw721
            .unwrap()
            .get(&Addr::unchecked("address2")),
        Some(&BTreeSet::from(["tokenid2".to_string()]))
    );

    let diff_balance = balance_a.difference(&balance_b).unwrap();
    assert!(diff_balance.is_empty());
}

#[test]
fn test_add_balances() {
    let balance_a = BalanceVerified {
        native: Some(BTreeMap::from([("token1".to_string(), Uint128::new(100))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("address1"),
            Uint128::new(200),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("address2"),
            BTreeSet::from(["tokenid1".to_string()]),
        )])),
    };

    let balance_b = BalanceVerified {
        native: Some(BTreeMap::from([
            ("token1".to_string(), Uint128::new(50)),
            ("token2".to_string(), Uint128::new(75)),
        ])),
        cw20: Some(BTreeMap::from([
            (Addr::unchecked("address1"), Uint128::new(100)),
            (Addr::unchecked("address3"), Uint128::new(150)),
        ])),
        cw721: Some(BTreeMap::from([
            (
                Addr::unchecked("address2"),
                BTreeSet::from(["tokenid2".to_string()]),
            ),
            (
                Addr::unchecked("address4"),
                BTreeSet::from(["tokenid3".to_string()]),
            ),
        ])),
    };

    let new_balance = balance_a.checked_add(&balance_b).unwrap();

    assert_eq!(
        new_balance.native,
        Some(BTreeMap::from([
            ("token1".to_string(), Uint128::new(150)),
            ("token2".to_string(), Uint128::new(75)),
        ]))
    );
    assert_eq!(
        new_balance.cw20,
        Some(BTreeMap::from([
            (Addr::unchecked("address1"), Uint128::new(300)),
            (Addr::unchecked("address3"), Uint128::new(150)),
        ]))
    );
    assert_eq!(
        new_balance.cw721,
        Some(BTreeMap::from([
            (
                Addr::unchecked("address2"),
                BTreeSet::from(["tokenid1".to_string(), "tokenid2".to_string()])
            ),
            (
                Addr::unchecked("address4"),
                BTreeSet::from(["tokenid3".to_string()])
            ),
        ]))
    );
}

#[test]
fn test_subtract_balances() {
    let balance_a = BalanceVerified {
        native: Some(BTreeMap::from([
            ("token1".to_string(), Uint128::new(100)),
            ("token2".to_string(), Uint128::new(75)),
        ])),
        cw20: Some(BTreeMap::from([
            (Addr::unchecked("address1"), Uint128::new(200)),
            (Addr::unchecked("address3"), Uint128::new(150)),
        ])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("address2"),
            BTreeSet::from(["tokenid1".to_string(), "tokenid2".to_string()]),
        )])),
    };

    let balance_b = BalanceVerified {
        native: Some(BTreeMap::from([("token1".to_string(), Uint128::new(50))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("address1"),
            Uint128::new(100),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("address2"),
            BTreeSet::from(["tokenid1".to_string()]),
        )])),
    };

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();

    assert_eq!(
        new_balance.native,
        Some(BTreeMap::from([
            ("token1".to_string(), Uint128::new(50)),
            ("token2".to_string(), Uint128::new(75)),
        ]))
    );
    assert_eq!(
        new_balance.cw20,
        Some(BTreeMap::from([
            (Addr::unchecked("address1"), Uint128::new(100)),
            (Addr::unchecked("address3"), Uint128::new(150)),
        ]))
    );
    assert_eq!(
        new_balance.cw721,
        Some(BTreeMap::from([(
            Addr::unchecked("address2"),
            BTreeSet::from(["tokenid2".to_string()])
        ),]))
    );
}

#[test]
fn test_checked_sub_with_insufficient_balance() {
    let balance1 = BalanceVerified {
        native: Some(BTreeMap::from([("token".to_string(), Uint128::new(50))])),
        cw20: None,
        cw721: None,
    };
    let balance2 = BalanceVerified {
        native: Some(BTreeMap::from([("token".to_string(), Uint128::new(100))])),
        cw20: None,
        cw721: None,
    };

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        BalanceError::InsufficientBalance
    ));
}

#[test]
fn test_checked_mul_with_zero() {
    let balance = BalanceVerified {
        native: Some(BTreeMap::from([("token".to_string(), Uint128::new(100))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("cw20"),
            Uint128::new(200),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("cw721"),
            BTreeSet::from(["1".to_string(), "2".to_string()]),
        )])),
    };

    let result = balance.checked_mul_floor(Decimal::zero()).unwrap();
    assert!(result.native.is_none());
    assert!(result.cw20.is_none());
    assert!(result.cw721.is_none());
}

#[test]
fn test_split_with_invalid_distribution() {
    let balance = BalanceVerified {
        native: Some(BTreeMap::from([("token".to_string(), Uint128::new(100))])),
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

    let result = balance1.difference(&balance2).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_checked_add_with_different_token_types() {
    let balance1 = BalanceVerified {
        native: Some(BTreeMap::from([("token1".to_string(), Uint128::new(100))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("cw20_1"),
            Uint128::new(200),
        )])),
        cw721: None,
    };
    let balance2 = BalanceVerified {
        native: Some(BTreeMap::from([("token2".to_string(), Uint128::new(50))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("cw20_2"),
            Uint128::new(150),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("cw721"),
            BTreeSet::from(["1".to_string()]),
        )])),
    };

    let result = balance1.checked_add(&balance2).unwrap();
    assert_eq!(result.native.unwrap().len(), 2);
    assert_eq!(result.cw20.unwrap().len(), 2);
    assert_eq!(result.cw721.unwrap().len(), 1);
}

#[test]
fn test_checked_sub_with_partial_amounts() {
    let balance1 = BalanceVerified {
        native: Some(BTreeMap::from([("token".to_string(), Uint128::new(100))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("cw20"),
            Uint128::new(200),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("cw721"),
            BTreeSet::from(["1".to_string(), "2".to_string()]),
        )])),
    };
    let balance2 = BalanceVerified {
        native: Some(BTreeMap::from([("token".to_string(), Uint128::new(60))])),
        cw20: Some(BTreeMap::from([(
            Addr::unchecked("cw20"),
            Uint128::new(50),
        )])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("cw721"),
            BTreeSet::from(["1".to_string()]),
        )])),
    };

    let result = balance1.checked_sub(&balance2).unwrap();
    assert_eq!(result.native.unwrap().get("token"), Some(&Uint128::new(40)));
    assert_eq!(
        result.cw20.unwrap().get(&Addr::unchecked("cw20")),
        Some(&Uint128::new(150))
    );
    assert_eq!(
        result.cw721.unwrap().get(&Addr::unchecked("cw721")),
        Some(&BTreeSet::from(["2".to_string()]))
    );
}
