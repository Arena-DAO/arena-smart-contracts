use crate::*;
use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw20::Cw20Coin;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

#[test]
fn test_balance_unchecked_into_checked() {
    let mock_deps = mock_dependencies();
    let deps = mock_deps.as_ref();

    // Test empty balance
    let balance = BalanceUnchecked {
        native: vec![],
        cw20: vec![],
        cw721: vec![],
    };
    let verified = balance.into_checked(deps).unwrap();
    assert!(verified.is_empty());

    let cw20_contract1 = mock_deps.api.addr_make("cw20_contract1");
    let cw20_contract2 = mock_deps.api.addr_make("cw20_contract2");
    let cw721_contract1 = mock_deps.api.addr_make("cw721_contract1");
    // Test full balance
    let balance = BalanceUnchecked {
        native: vec![
            Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(100),
            },
            Coin {
                denom: "uosmo".to_string(),
                amount: Uint128::new(50),
            },
        ],
        cw20: vec![
            Cw20Coin {
                address: cw20_contract1.to_string(),
                amount: Uint128::new(75),
            },
            Cw20Coin {
                address: cw20_contract2.to_string(),
                amount: Uint128::new(25),
            },
        ],
        cw721: vec![Cw721Collection {
            address: cw721_contract1.to_string(),
            token_ids: vec!["token1".to_string(), "token2".to_string()],
        }],
    };
    let verified = balance.into_checked(deps).unwrap();

    // Check native balance
    assert_eq!(verified.native.len(), 2);
    assert_eq!(verified.native.get("uatom"), Some(&Uint128::new(100)));
    assert_eq!(verified.native.get("uosmo"), Some(&Uint128::new(50)));

    // Check cw20 balance
    assert_eq!(verified.cw20.len(), 2);
    assert_eq!(verified.cw20.get(&cw20_contract1), Some(&Uint128::new(75)));
    assert_eq!(verified.cw20.get(&cw20_contract2), Some(&Uint128::new(25)));

    // Check cw721 balance
    assert_eq!(verified.cw721.len(), 1);
    let token_set = verified
        .cw721
        .get(&Addr::unchecked(cw721_contract1))
        .unwrap();
    assert_eq!(token_set.len(), 2);
    assert!(token_set.contains("token1"));
    assert!(token_set.contains("token2"));
}

#[test]
fn test_balance_verified_is_empty() {
    // Empty balance
    let balance = BalanceVerified {
        native: BTreeMap::new(),
        cw20: BTreeMap::new(),
        cw721: BTreeMap::new(),
    };
    assert!(balance.is_empty());

    // Non-empty native
    let mut balance = BalanceVerified {
        native: BTreeMap::new(),
        cw20: BTreeMap::new(),
        cw721: BTreeMap::new(),
    };
    balance
        .native
        .insert("uatom".to_string(), Uint128::new(100));
    assert!(!balance.is_empty());

    // Non-empty cw20
    let mut balance = BalanceVerified {
        native: BTreeMap::new(),
        cw20: BTreeMap::new(),
        cw721: BTreeMap::new(),
    };
    balance
        .cw20
        .insert(Addr::unchecked("cw20_contract"), Uint128::new(100));
    assert!(!balance.is_empty());

    // Non-empty cw721
    let mut balance = BalanceVerified {
        native: BTreeMap::new(),
        cw20: BTreeMap::new(),
        cw721: BTreeMap::new(),
    };
    let mut token_set = BTreeSet::new();
    token_set.insert("token1".to_string());
    balance
        .cw721
        .insert(Addr::unchecked("cw721_contract"), token_set);
    assert!(!balance.is_empty());
}

#[test]
fn test_balance_verified_checked_add() {
    // Create two balances to add
    let mut balance1 = BalanceVerified::default();
    balance1
        .native
        .insert("uatom".to_string(), Uint128::new(100));
    balance1
        .cw20
        .insert(Addr::unchecked("cw20_contract1"), Uint128::new(75));
    let mut token_set1 = BTreeSet::new();
    token_set1.insert("token1".to_string());
    token_set1.insert("token2".to_string());
    balance1
        .cw721
        .insert(Addr::unchecked("cw721_contract1"), token_set1);

    let mut balance2 = BalanceVerified::default();
    balance2
        .native
        .insert("uatom".to_string(), Uint128::new(50));
    balance2
        .native
        .insert("uosmo".to_string(), Uint128::new(25));
    balance2
        .cw20
        .insert(Addr::unchecked("cw20_contract1"), Uint128::new(25));
    balance2
        .cw20
        .insert(Addr::unchecked("cw20_contract2"), Uint128::new(10));
    let mut token_set2 = BTreeSet::new();
    token_set2.insert("token3".to_string());
    balance2
        .cw721
        .insert(Addr::unchecked("cw721_contract1"), token_set2);
    let mut token_set3 = BTreeSet::new();
    token_set3.insert("tokenA".to_string());
    balance2
        .cw721
        .insert(Addr::unchecked("cw721_contract2"), token_set3);

    // Add the balances
    let result = balance1.checked_add(&balance2).unwrap();

    // Check native coins
    assert_eq!(result.native.len(), 2);
    assert_eq!(result.native.get("uatom"), Some(&Uint128::new(150)));
    assert_eq!(result.native.get("uosmo"), Some(&Uint128::new(25)));

    // Check cw20 tokens
    assert_eq!(result.cw20.len(), 2);
    assert_eq!(
        result.cw20.get(&Addr::unchecked("cw20_contract1")),
        Some(&Uint128::new(100))
    );
    assert_eq!(
        result.cw20.get(&Addr::unchecked("cw20_contract2")),
        Some(&Uint128::new(10))
    );

    // Check cw721 tokens
    assert_eq!(result.cw721.len(), 2);
    let token_set = result
        .cw721
        .get(&Addr::unchecked("cw721_contract1"))
        .unwrap();
    assert_eq!(token_set.len(), 3);
    assert!(token_set.contains("token1"));
    assert!(token_set.contains("token2"));
    assert!(token_set.contains("token3"));
    let token_set = result
        .cw721
        .get(&Addr::unchecked("cw721_contract2"))
        .unwrap();
    assert_eq!(token_set.len(), 1);
    assert!(token_set.contains("tokenA"));
}

#[test]
fn test_balance_verified_checked_sub() {
    // Create two balances for subtraction
    let mut balance1 = BalanceVerified::default();
    balance1
        .native
        .insert("uatom".to_string(), Uint128::new(100));
    balance1
        .native
        .insert("uosmo".to_string(), Uint128::new(50));
    balance1
        .cw20
        .insert(Addr::unchecked("cw20_contract1"), Uint128::new(75));
    balance1
        .cw20
        .insert(Addr::unchecked("cw20_contract2"), Uint128::new(25));
    let mut token_set1 = BTreeSet::new();
    token_set1.insert("token1".to_string());
    token_set1.insert("token2".to_string());
    token_set1.insert("token3".to_string());
    balance1
        .cw721
        .insert(Addr::unchecked("cw721_contract1"), token_set1);

    let mut balance2 = BalanceVerified::default();
    balance2
        .native
        .insert("uatom".to_string(), Uint128::new(50));
    balance2
        .cw20
        .insert(Addr::unchecked("cw20_contract1"), Uint128::new(25));
    let mut token_set2 = BTreeSet::new();
    token_set2.insert("token1".to_string());
    balance2
        .cw721
        .insert(Addr::unchecked("cw721_contract1"), token_set2);

    // Subtract the balances
    let result = balance1.checked_sub(&balance2).unwrap();

    // Check native coins
    assert_eq!(result.native.len(), 2);
    assert_eq!(result.native.get("uatom"), Some(&Uint128::new(50)));
    assert_eq!(result.native.get("uosmo"), Some(&Uint128::new(50)));

    // Check cw20 tokens
    assert_eq!(result.cw20.len(), 2);
    assert_eq!(
        result.cw20.get(&Addr::unchecked("cw20_contract1")),
        Some(&Uint128::new(50))
    );
    assert_eq!(
        result.cw20.get(&Addr::unchecked("cw20_contract2")),
        Some(&Uint128::new(25))
    );

    // Check cw721 tokens
    assert_eq!(result.cw721.len(), 1);
    let token_set = result
        .cw721
        .get(&Addr::unchecked("cw721_contract1"))
        .unwrap();
    assert_eq!(token_set.len(), 2);
    assert!(!token_set.contains("token1"));
    assert!(token_set.contains("token2"));
    assert!(token_set.contains("token3"));

    // Test complete removal of an entry when it becomes zero
    let mut balance3 = BalanceVerified::default();
    balance3
        .native
        .insert("uatom".to_string(), Uint128::new(50));
    balance3
        .cw20
        .insert(Addr::unchecked("cw20_contract2"), Uint128::new(25));
    let mut token_set3 = BTreeSet::new();
    token_set3.insert("token2".to_string());
    token_set3.insert("token3".to_string());
    balance3
        .cw721
        .insert(Addr::unchecked("cw721_contract1"), token_set3);

    let result = result.checked_sub(&balance3).unwrap();
    assert!(!result.native.contains_key("uatom"));
    assert!(result.native.contains_key("uosmo"));
    assert!(!result.cw20.contains_key(&Addr::unchecked("cw20_contract2")));
    assert!(result.cw20.contains_key(&Addr::unchecked("cw20_contract1")));
    assert!(!result
        .cw721
        .contains_key(&Addr::unchecked("cw721_contract1")));
}

#[test]
fn test_balance_verified_checked_sub_errors() {
    // Test insufficient native balance
    let mut balance1 = BalanceVerified::default();
    balance1
        .native
        .insert("uatom".to_string(), Uint128::new(50));

    let mut balance2 = BalanceVerified::default();
    balance2
        .native
        .insert("uatom".to_string(), Uint128::new(100));

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Insufficient native balance"));

    // Test missing native denom
    let mut balance1 = BalanceVerified::default();
    balance1
        .native
        .insert("uatom".to_string(), Uint128::new(50));

    let mut balance2 = BalanceVerified::default();
    balance2
        .native
        .insert("uosmo".to_string(), Uint128::new(50));

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing native denom"));

    // Test insufficient cw20 balance
    let mut balance1 = BalanceVerified::default();
    balance1
        .cw20
        .insert(Addr::unchecked("cw20_contract"), Uint128::new(50));

    let mut balance2 = BalanceVerified::default();
    balance2
        .cw20
        .insert(Addr::unchecked("cw20_contract"), Uint128::new(100));

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Insufficient CW20 balance"));

    // Test missing cw20 token
    let mut balance1 = BalanceVerified::default();
    balance1
        .cw20
        .insert(Addr::unchecked("cw20_contract1"), Uint128::new(50));

    let mut balance2 = BalanceVerified::default();
    balance2
        .cw20
        .insert(Addr::unchecked("cw20_contract2"), Uint128::new(50));

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing CW20 token"));

    // Test missing cw721 token
    let mut balance1 = BalanceVerified::default();
    let mut token_set1 = BTreeSet::new();
    token_set1.insert("token1".to_string());
    balance1
        .cw721
        .insert(Addr::unchecked("cw721_contract"), token_set1);

    let mut balance2 = BalanceVerified::default();
    let mut token_set2 = BTreeSet::new();
    token_set2.insert("token2".to_string());
    balance2
        .cw721
        .insert(Addr::unchecked("cw721_contract"), token_set2);

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing CW721 token"));

    // Test missing cw721 collection
    let mut balance1 = BalanceVerified::default();
    let mut token_set1 = BTreeSet::new();
    token_set1.insert("token1".to_string());
    balance1
        .cw721
        .insert(Addr::unchecked("cw721_contract1"), token_set1);

    let mut balance2 = BalanceVerified::default();
    let mut token_set2 = BTreeSet::new();
    token_set2.insert("token1".to_string());
    balance2
        .cw721
        .insert(Addr::unchecked("cw721_contract2"), token_set2);

    let result = balance1.checked_sub(&balance2);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing CW721 collection"));
}

#[test]
fn test_balance_verified_checked_mul_floor() {
    // Create a balance to multiply
    let mut balance = BalanceVerified::default();
    balance
        .native
        .insert("uatom".to_string(), Uint128::new(100));
    balance.native.insert("uosmo".to_string(), Uint128::new(55));
    balance
        .cw20
        .insert(Addr::unchecked("cw20_contract"), Uint128::new(75));
    let mut token_set = BTreeSet::new();
    token_set.insert("token1".to_string());
    token_set.insert("token2".to_string());
    balance
        .cw721
        .insert(Addr::unchecked("cw721_contract"), token_set);

    // Test multiply by zero
    let result = balance.checked_mul_floor(Decimal::zero()).unwrap();
    assert!(result.is_empty());

    // Test multiply by one
    let result = balance.checked_mul_floor(Decimal::one()).unwrap();
    assert_eq!(result.native.get("uatom"), Some(&Uint128::new(100)));
    assert_eq!(result.native.get("uosmo"), Some(&Uint128::new(55)));
    assert_eq!(
        result.cw20.get(&Addr::unchecked("cw20_contract")),
        Some(&Uint128::new(75))
    );
    assert_eq!(result.cw721.len(), 1);

    // Test multiply by fraction
    let half = Decimal::from_str("0.5").unwrap();
    let result = balance.checked_mul_floor(half).unwrap();
    assert_eq!(result.native.get("uatom"), Some(&Uint128::new(50)));
    assert_eq!(result.native.get("uosmo"), Some(&Uint128::new(27))); // 55 * 0.5 = 27.5, floored to 27
    assert_eq!(
        result.cw20.get(&Addr::unchecked("cw20_contract")),
        Some(&Uint128::new(37))
    ); // 75 * 0.5 = 37.5, floored to 37
    assert_eq!(result.cw721.len(), 0); // NFTs are handled differently: only get all or none
}

#[test]
fn test_balance_verified_split() {
    // Create a balance to split
    let mut balance = BalanceVerified::default();
    balance
        .native
        .insert("uatom".to_string(), Uint128::new(100));
    balance.native.insert("uosmo".to_string(), Uint128::new(50));
    balance
        .cw20
        .insert(Addr::unchecked("cw20_contract"), Uint128::new(75));
    let mut token_set = BTreeSet::new();
    token_set.insert("token1".to_string());
    token_set.insert("token2".to_string());
    balance
        .cw721
        .insert(Addr::unchecked("cw721_contract"), token_set);

    // Create a distribution
    let distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: Addr::unchecked("member1"),
                percentage: Decimal::from_str("0.3").unwrap(), // 30%
            },
            MemberPercentage {
                addr: Addr::unchecked("member2"),
                percentage: Decimal::from_str("0.2").unwrap(), // 20%
            },
        ],
        remainder_addr: Addr::unchecked("remainder"),
    };

    // Split the balance
    let result = BalanceVerified::split(&balance, &distribution, &BTreeMap::new()).unwrap();

    // The percentages add up to 50%, so 50% should go to remainder
    assert_eq!(result.len(), 3);

    // Find member1's balance (30%)
    let member1 = result
        .iter()
        .find(|m| m.addr == Addr::unchecked("member1"))
        .unwrap();
    assert_eq!(member1.balance.native.get("uatom"), Some(&Uint128::new(30)));
    assert_eq!(member1.balance.native.get("uosmo"), Some(&Uint128::new(15)));
    assert_eq!(
        member1.balance.cw20.get(&Addr::unchecked("cw20_contract")),
        Some(&Uint128::new(22))
    );
    assert!(member1.balance.cw721.is_empty());

    // Find member2's balance (20%)
    let member2 = result
        .iter()
        .find(|m| m.addr == Addr::unchecked("member2"))
        .unwrap();
    assert_eq!(member2.balance.native.get("uatom"), Some(&Uint128::new(20)));
    assert_eq!(member2.balance.native.get("uosmo"), Some(&Uint128::new(10)));
    assert_eq!(
        member2.balance.cw20.get(&Addr::unchecked("cw20_contract")),
        Some(&Uint128::new(15))
    );
    assert!(member2.balance.cw721.is_empty());

    // Find remainder balance (50%)
    let remainder = result
        .iter()
        .find(|m| m.addr == Addr::unchecked("remainder"))
        .unwrap();
    assert_eq!(
        remainder.balance.native.get("uatom"),
        Some(&Uint128::new(50))
    );
    assert_eq!(
        remainder.balance.native.get("uosmo"),
        Some(&Uint128::new(25))
    );
    assert_eq!(
        remainder
            .balance
            .cw20
            .get(&Addr::unchecked("cw20_contract")),
        Some(&Uint128::new(38))
    );
    assert_eq!(remainder.balance.cw721.len(), 1);
    let token_set = remainder
        .balance
        .cw721
        .get(&Addr::unchecked("cw721_contract"))
        .unwrap();
    assert_eq!(token_set.len(), 2);
}

#[test]
fn test_split_with_cw721_routing_and_100_percent_member() {
    // === TEST 1: Normal split with remainder and routing ===
    let mut balance = BalanceVerified::default();

    let mut token_set = BTreeSet::new();
    token_set.insert("nft1".to_string());
    token_set.insert("nft2".to_string());
    token_set.insert("nft3".to_string());
    balance
        .cw721
        .insert(Addr::unchecked("nft_contract"), token_set.clone());

    let distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: Addr::unchecked("member1"),
                percentage: Decimal::percent(50),
            },
            MemberPercentage {
                addr: Addr::unchecked("member2"),
                percentage: Decimal::percent(30),
            },
        ],
        remainder_addr: Addr::unchecked("remainder"),
    };

    // nft1 → member1, nft2 → member2, nft3 → remainder
    let mut nft_owners = BTreeMap::new();
    nft_owners
        .entry(Addr::unchecked("member1"))
        .or_insert_with(BTreeMap::new)
        .entry(Addr::unchecked("nft_contract"))
        .or_insert_with(BTreeSet::new)
        .insert("nft1".to_string());

    nft_owners
        .entry(Addr::unchecked("member2"))
        .or_insert_with(BTreeMap::new)
        .entry(Addr::unchecked("nft_contract"))
        .or_insert_with(BTreeSet::new)
        .insert("nft2".to_string());

    let result = BalanceVerified::split(&balance, &distribution, &nft_owners).unwrap();
    assert_eq!(result.len(), 3); // member1, member2, remainder

    let tokens = |r: &MemberBalanceChecked, id| {
        r.balance
            .cw721
            .get(&Addr::unchecked("nft_contract"))
            .map(|s| s.contains(id))
            .unwrap_or(false)
    };

    let member1 = result
        .iter()
        .find(|r| r.addr.as_ref() == "member1")
        .unwrap();
    assert!(tokens(member1, "nft1"));
    assert!(!tokens(member1, "nft2"));
    assert!(!tokens(member1, "nft3"));

    let member2 = result
        .iter()
        .find(|r| r.addr.as_ref() == "member2")
        .unwrap();
    assert!(tokens(member2, "nft2"));
    assert!(!tokens(member2, "nft1"));
    assert!(!tokens(member2, "nft3"));

    let remainder = result
        .iter()
        .find(|r| r.addr.as_ref() == "remainder")
        .unwrap();
    assert!(tokens(remainder, "nft3"));

    // === TEST 2: 100% to one member → all NFTs should go there, ownership map ignored ===
    let distribution_all = Distribution {
        member_percentages: vec![MemberPercentage {
            addr: Addr::unchecked("member1"),
            percentage: Decimal::percent(100),
        }],
        remainder_addr: Addr::unchecked("remainder"), // irrelevant
    };

    let result_all = BalanceVerified::split(&balance, &distribution_all, &nft_owners).unwrap();

    assert_eq!(result_all.len(), 1);
    let only = &result_all[0];
    assert_eq!(only.addr, Addr::unchecked("member1"));

    let member1_tokens = only
        .balance
        .cw721
        .get(&Addr::unchecked("nft_contract"))
        .unwrap();
    assert!(member1_tokens.contains("nft1"));
    assert!(member1_tokens.contains("nft2"));
    assert!(member1_tokens.contains("nft3"));
}
