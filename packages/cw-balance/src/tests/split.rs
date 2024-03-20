use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw20::Cw20CoinVerified;

use crate::{BalanceVerified, Cw721CollectionVerified, Distribution, MemberPercentage};

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
            address: Addr::unchecked("cw721token1"),
            token_ids: vec!["1".to_string(), "2".to_string()],
        }],
    };

    let distribution = Distribution::<Addr> {
        member_percentages: vec![
            MemberPercentage {
                addr: addr_a.clone(),
                percentage: Decimal::from_ratio(50u128, 80u128),
            },
            MemberPercentage {
                addr: addr_b.clone(),
                percentage: Decimal::from_ratio(30u128, 80u128),
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
        .find(|coin| coin.address == Addr::unchecked("cw721token1"))
        .unwrap();
    assert_eq!(remainder_cw20token1.amount, Uint128::from(1u128));
    assert_eq!(remainder_cw20token2.amount, Uint128::from(1u128));
    assert_eq!(remainder_cw721token1.token_ids, vec!["1", "2"]);
}

#[test]
fn test_split_balances_with_remainder_as_member_share() {
    let addr_a = Addr::unchecked("addr_a");
    let addr_b = Addr::unchecked("addr_b");
    let addr_c = Addr::unchecked("addr_c");

    let balance = BalanceVerified {
        native: vec![Coin {
            denom: "native1".to_string(),
            amount: Uint128::from(100u128),
        }],
        cw20: vec![],
        cw721: vec![Cw721CollectionVerified {
            address: Addr::unchecked("cw721"),
            token_ids: vec!["1".to_string()],
        }],
    };

    let distribution = Distribution::<Addr> {
        member_percentages: vec![
            MemberPercentage {
                addr: addr_a.clone(),
                percentage: Decimal::from_ratio(33u128, 100u128),
            },
            MemberPercentage {
                addr: addr_b.clone(),
                percentage: Decimal::from_ratio(33u128, 100u128),
            },
            MemberPercentage {
                addr: addr_c.clone(),
                percentage: Decimal::from_ratio(34u128, 100u128),
            },
        ],
        remainder_addr: addr_c.clone(),
    };

    let split_result = balance.split(&distribution).unwrap();

    assert_eq!(split_result[0].balance.native[0].amount.u128(), 33u128);
    assert_eq!(split_result[1].balance.native[0].amount.u128(), 33u128);
    assert_eq!(split_result[2].balance.native[0].amount.u128(), 34u128);
    assert_eq!(split_result[2].balance.cw721[0].token_ids, vec!["1"]);
}
