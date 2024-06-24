use crate::{BalanceVerified, Distribution, MemberPercentage};
use cosmwasm_std::{Addr, Decimal, Uint128};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn test_split_balances() {
    let addr_a = Addr::unchecked("addr_a");
    let addr_b = Addr::unchecked("addr_b");
    let addr_c = Addr::unchecked("addr_c");

    let balance = BalanceVerified {
        native: Some(BTreeMap::from([
            ("native1".to_string(), Uint128::new(200)),
            ("native2".to_string(), Uint128::new(100)),
        ])),
        cw20: Some(BTreeMap::from([
            (Addr::unchecked("cw20token1"), Uint128::new(150)),
            (Addr::unchecked("cw20token2"), Uint128::new(50)),
        ])),
        cw721: Some(BTreeMap::from([(
            Addr::unchecked("cw721token1"),
            BTreeSet::from(["1".to_string(), "2".to_string()]),
        )])),
    };

    let distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: addr_a.clone(),
                percentage: Decimal::percent(60),
            },
            MemberPercentage {
                addr: addr_b.clone(),
                percentage: Decimal::percent(40),
            },
        ],
        remainder_addr: addr_c.clone(),
    };

    let split_result = balance.split(&distribution).unwrap();
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

    // Check member A's balance (60%)
    assert_eq!(
        member_a_balance.native.as_ref().unwrap().get("native1"),
        Some(&Uint128::new(120))
    );
    assert_eq!(
        member_a_balance.native.as_ref().unwrap().get("native2"),
        Some(&Uint128::new(60))
    );
    assert_eq!(
        member_a_balance
            .cw20
            .as_ref()
            .unwrap()
            .get(&Addr::unchecked("cw20token1")),
        Some(&Uint128::new(90))
    );
    assert_eq!(
        member_a_balance
            .cw20
            .as_ref()
            .unwrap()
            .get(&Addr::unchecked("cw20token2")),
        Some(&Uint128::new(30))
    );

    // Check member B's balance (40%)
    assert_eq!(
        member_b_balance.native.as_ref().unwrap().get("native1"),
        Some(&Uint128::new(80))
    );
    assert_eq!(
        member_b_balance.native.as_ref().unwrap().get("native2"),
        Some(&Uint128::new(40))
    );
    assert_eq!(
        member_b_balance
            .cw20
            .as_ref()
            .unwrap()
            .get(&Addr::unchecked("cw20token1")),
        Some(&Uint128::new(60))
    );
    assert_eq!(
        member_b_balance
            .cw20
            .as_ref()
            .unwrap()
            .get(&Addr::unchecked("cw20token2")),
        Some(&Uint128::new(20))
    );

    // Check remainder balance (should be empty or have minimal remainders due to rounding)
    assert!(
        remainder_balance.native.is_none() || remainder_balance.native.as_ref().unwrap().is_empty()
    );
    assert!(
        remainder_balance.cw20.is_none() || remainder_balance.cw20.as_ref().unwrap().is_empty()
    );
    assert_eq!(
        remainder_balance
            .cw721
            .as_ref()
            .unwrap()
            .get(&Addr::unchecked("cw721token1")),
        Some(&BTreeSet::from(["1".to_string(), "2".to_string()]))
    );
}

// Add a new test for invalid percentages
#[test]
fn test_split_balances_invalid_percentages() {
    let addr_a = Addr::unchecked("addr_a");
    let addr_b = Addr::unchecked("addr_b");
    let addr_c = Addr::unchecked("addr_c");

    let balance = BalanceVerified {
        native: Some(BTreeMap::from([("native1".to_string(), Uint128::new(100))])),
        cw20: None,
        cw721: None,
    };

    let distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: addr_a.clone(),
                percentage: Decimal::percent(60),
            },
            MemberPercentage {
                addr: addr_b.clone(),
                percentage: Decimal::percent(60),
            },
        ],
        remainder_addr: addr_c,
    };

    let result = balance.split(&distribution);
    assert!(result.is_err());
}
