use arena_interface::registry::{ExecuteMsgFns as _, QueryMsgFns as _};
use cosmwasm_std::Decimal;
use cw_balance::{Distribution, MemberPercentage};
use cw_orch::{anyhow, prelude::*};

use crate::tests::helpers::setup_arena;

use super::PREFIX;

#[test]
fn test_set_and_get_distribution() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make("user1");
    let user2 = mock.addr_make("user2");

    arena.arena_payment_registry.set_sender(&admin);

    // Set a distribution
    let distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::percent(60),
            },
            MemberPercentage {
                addr: user2.to_string(),
                percentage: Decimal::percent(40),
            },
        ],
        remainder_addr: admin.to_string(),
    };

    let res = arena
        .arena_payment_registry
        .set_distribution(distribution.clone())?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "set_distribution")));
    mock.next_block()?;

    // Query the distribution
    let queried_distribution = arena
        .arena_payment_registry
        .get_distribution(admin.to_string(), None)?;

    assert_eq!(queried_distribution, Some(distribution));

    Ok(())
}

#[test]
fn test_remove_distribution() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make("user1");
    let user2 = mock.addr_make("user2");

    arena.arena_payment_registry.set_sender(&admin);

    // Set a distribution
    let distribution = Distribution {
        member_percentages: vec![
            MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::percent(60),
            },
            MemberPercentage {
                addr: user2.to_string(),
                percentage: Decimal::percent(40),
            },
        ],
        remainder_addr: admin.to_string(),
    };

    arena
        .arena_payment_registry
        .set_distribution(distribution)?;

    // Remove the distribution
    arena.arena_payment_registry.remove_distribution()?;
    mock.next_block()?;

    // Query the distribution (should be None)
    let queried_distribution = arena
        .arena_payment_registry
        .get_distribution(admin.to_string(), None)?;

    assert_eq!(queried_distribution, None);

    Ok(())
}

#[test]
fn test_multiple_users_distributions() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (arena, _admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make("user1");
    let user2 = mock.addr_make("user2");

    // Set distribution for user1
    let distribution1 = Distribution {
        member_percentages: vec![MemberPercentage {
            addr: user1.to_string(),
            percentage: Decimal::percent(100),
        }],
        remainder_addr: user1.to_string(),
    };
    arena
        .arena_payment_registry
        .call_as(&user1)
        .set_distribution(distribution1.clone())?;

    // Set distribution for user2
    let distribution2 = Distribution {
        member_percentages: vec![MemberPercentage {
            addr: user2.to_string(),
            percentage: Decimal::percent(100),
        }],
        remainder_addr: user2.to_string(),
    };
    arena
        .arena_payment_registry
        .call_as(&user2)
        .set_distribution(distribution2.clone())?;
    mock.next_block()?;

    // Query distributions for both users
    let queried_distribution1 = arena
        .arena_payment_registry
        .get_distribution(user1.to_string(), None)?;
    let queried_distribution2 = arena
        .arena_payment_registry
        .get_distribution(user2.to_string(), None)?;

    assert_eq!(queried_distribution1, Some(distribution1));
    assert_eq!(queried_distribution2, Some(distribution2));

    Ok(())
}
