use crate::cw721::Cw721CollectionVerified;
use crate::{BalanceVerified, Distribution, MemberPercentage};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw20::Cw20CoinVerified;

#[test]
fn test_split_balances() {
    let addr_a = Addr::unchecked("addr_a");
    let addr_b = Addr::unchecked("addr_b");
    let addr_c = Addr::unchecked("addr_c");

    let balance = BalanceVerified {
        native: Some(vec![
            Coin {
                denom: "native1".to_string(),
                amount: Uint128::new(200),
            },
            Coin {
                denom: "native2".to_string(),
                amount: Uint128::new(100),
            },
        ]),
        cw20: Some(vec![
            Cw20CoinVerified {
                address: Addr::unchecked("cw20token1"),
                amount: Uint128::new(150),
            },
            Cw20CoinVerified {
                address: Addr::unchecked("cw20token2"),
                amount: Uint128::new(50),
            },
        ]),
        cw721: Some(vec![Cw721CollectionVerified {
            address: Addr::unchecked("cw721token1"),
            token_ids: vec!["1".to_string(), "2".to_string()],
        }]),
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
        member_a_balance
            .native
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.denom == "native1")
            .map(|c| c.amount),
        Some(Uint128::new(120))
    );
    assert_eq!(
        member_a_balance
            .native
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.denom == "native2")
            .map(|c| c.amount),
        Some(Uint128::new(60))
    );
    assert_eq!(
        member_a_balance
            .cw20
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("cw20token1"))
            .map(|c| c.amount),
        Some(Uint128::new(90))
    );
    assert_eq!(
        member_a_balance
            .cw20
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("cw20token2"))
            .map(|c| c.amount),
        Some(Uint128::new(30))
    );

    // Check member B's balance (40%)
    assert_eq!(
        member_b_balance
            .native
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.denom == "native1")
            .map(|c| c.amount),
        Some(Uint128::new(80))
    );
    assert_eq!(
        member_b_balance
            .native
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.denom == "native2")
            .map(|c| c.amount),
        Some(Uint128::new(40))
    );
    assert_eq!(
        member_b_balance
            .cw20
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("cw20token1"))
            .map(|c| c.amount),
        Some(Uint128::new(60))
    );
    assert_eq!(
        member_b_balance
            .cw20
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.address == Addr::unchecked("cw20token2"))
            .map(|c| c.amount),
        Some(Uint128::new(20))
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
            .first()
            .map(|c| &c.token_ids),
        Some(&vec!["1".to_string(), "2".to_string()])
    );
}

#[test]
fn test_split_balances_invalid_percentages() {
    let addr_a = Addr::unchecked("addr_a");
    let addr_b = Addr::unchecked("addr_b");
    let addr_c = Addr::unchecked("addr_c");

    let balance = BalanceVerified {
        native: Some(vec![Coin {
            denom: "native1".to_string(),
            amount: Uint128::new(100),
        }]),
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
