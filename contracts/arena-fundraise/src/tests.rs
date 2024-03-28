use std::str::FromStr;

use cosmwasm_std::{Addr, Coin, Coins, Empty, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::{Duration, Expiration};

use crate::{
    contract,
    msg::{ExecuteMsg, InstantiateMsg},
};

struct Context {
    app: App,
    fundraise: Addr,
}

fn arena_fundraise_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        contract::execute,
        contract::instantiate,
        contract::query,
    ))
}

/// Fundraise 1m arena for 10k usdc up to 100k usdc with start at block + 10 and duration 100 blocks
fn setup(balances: &[(Addr, Coins)]) -> Context {
    let mut app = App::default();

    app.init_modules(|router, _, storage| {
        for balance in balances {
            router
                .bank
                .init_balance(storage, &balance.0, balance.1.clone().into_vec())
                .unwrap();
        }
    });

    let fundraise_code = app.store_code(arena_fundraise_contract());

    // Create a fundraiser that will distribute 1m arena in exchange for usdc
    let coins = balances[0].1.to_vec();
    let deposit_denom = &balances[1].1.denoms()[0];
    let fundraise = app
        .instantiate_contract(
            fundraise_code,
            balances[0].0.clone(),
            &InstantiateMsg {
                fundraise: coins[0].clone(),
                deposit_denom: deposit_denom.clone(),
                soft_cap: Uint128::new(10_000u128),
                hard_cap: Some(Uint128::new(100_000u128)),
                start: Some(Expiration::AtHeight(12355)),
                duration: Duration::Height(100u64),
                recipient: balances[0].0.to_string(),
            },
            &coins,
            "Arena Fundraise",
            None,
        )
        .unwrap();

    Context { app, fundraise }
}

fn get_basic_balances() -> Vec<(Addr, Coins)> {
    let dao = Addr::unchecked("dao");
    let mut users = vec![];
    for i in 0..10 {
        users.push(Addr::unchecked(format!("user{}", i)));
    }

    let mut balances = vec![(dao, Coins::from_str("1000000arena").unwrap())];

    for user in users {
        balances.push((user, Coins::from_str("10000usdc").unwrap()));
    }

    balances
}

#[test]
fn test_success() {
    let balances = get_basic_balances();

    let mut context = setup(&balances);

    let coin = Coin {
        denom: "usdc".to_string(),
        amount: Uint128::new(5_000u128),
    };

    // Execute fails - not started
    let response = context.app.execute_contract(
        balances[1].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Deposit {},
        &[coin.clone()],
    );
    assert!(response.is_err());

    context.app.update_block(|x| x.height += 20);

    for i in 0..10 {
        // Deposit success after start
        let response = context.app.execute_contract(
            balances[i + 1].0.clone(),
            context.fundraise.clone(),
            &ExecuteMsg::Deposit {},
            &[coin.clone()],
        );
        assert!(response.is_ok());
    }

    context.app.update_block(|x| x.height += 10000);

    // Execute fails - expired
    let response = context.app.execute_contract(
        balances[1].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Deposit {},
        &[coin.clone()],
    );
    assert!(response.is_err());

    // Execute success after end
    let response = context.app.execute_contract(
        balances[0].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Expire {},
        &[],
    );
    assert!(response.is_ok());

    // Query balance - 10 * 5k
    let balance = context
        .app
        .wrap()
        .query_balance(balances[0].0.clone(), "usdc")
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(50_000u128));

    // User can withdraw new token
    let response = context.app.execute_contract(
        balances[1].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    );
    assert!(response.is_ok());

    // Query balance - 1m / 10
    let balance = context
        .app
        .wrap()
        .query_balance(balances[1].0.clone(), "arena")
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(100_000u128));
}

#[test]
fn test_failure() {
    let balances = get_basic_balances();

    let mut context = setup(&balances);

    context.app.update_block(|x| x.height += 20);

    // Execute fails - not ended
    let response = context.app.execute_contract(
        balances[0].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Expire {},
        &[],
    );
    assert!(response.is_err());

    // Send 1 amount below soft cap
    let response = context.app.execute_contract(
        balances[1].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Deposit {},
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(5000),
        }],
    );
    assert!(response.is_ok());

    // rip no one else sent :(
    context.app.update_block(|x| x.height += 1000);

    // Execute success after end
    let response = context.app.execute_contract(
        balances[0].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Expire {},
        &[],
    );
    assert!(response.is_ok());

    // Query balance
    let balance = context
        .app
        .wrap()
        .query_balance(balances[0].0.clone(), "arena")
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(1_000_000u128));

    // Withdraw
    let response = context.app.execute_contract(
        balances[1].0.clone(),
        context.fundraise.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    );
    assert!(response.is_ok());

    // Query balance - 5k deposit + 5k wallet
    let balance = context
        .app
        .wrap()
        .query_balance(balances[1].0.clone(), "usdc")
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(10_000u128));
}
