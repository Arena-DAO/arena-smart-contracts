use std::collections::{BTreeMap, HashMap};

use cosmwasm_std::{from_binary, Addr, Binary, Decimal, Uint128};
use cw_balance::Distribution;
use cw_multi_test::{App, Executor};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

const CREATOR: &str = "creator";

fn setup() -> (App, Addr) {
    let mut app = App::default();
    let escrow_code_id = app.store_code(agon_testing::contracts::agon_escrow_contract());
    let contract_addr = app
        .instantiate_contract(
            escrow_code_id,
            Addr::unchecked(CREATOR),
            &InstantiateMsg {
                dues: HashMap::new(),
                stakes: HashMap::new(),
            },
            &vec![],
            "Agon Escrow",
            None,
        )
        .unwrap();
    (app, contract_addr)
}

#[test]
fn test_lock() {
    let (mut app, contract) = setup();

    // Try to withdraw when the contract is locked
    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract.clone(),
        &ExecuteMsg::Lock { value: true },
        &vec![],
    )
    .unwrap();

    let res = app.execute_contract(
        Addr::unchecked(CREATOR),
        contract.clone(),
        &ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &vec![],
    );
    assert_eq!(
        res.unwrap_err().root_cause().to_string(),
        ContractError::Locked {}.to_string()
    );

    // Try to withdraw when the contract is unlocked
    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract.clone(),
        &ExecuteMsg::Lock { value: false },
        &vec![],
    )
    .unwrap();

    let res = app.execute_contract(
        Addr::unchecked(CREATOR),
        contract.clone(),
        &ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &vec![],
    );
    assert!(res.is_ok());
}

#[test]
fn test_set_distribution() {
    let (mut app, contract) = setup();

    let mut distribution = BTreeMap::new();
    distribution.insert("addr1".to_string(), Decimal::new(Uint128::new(50u128)));
    distribution.insert("addr2".to_string(), Decimal::new(Uint128::new(30)));
    distribution.insert("addr3".to_string(), Decimal::new(Uint128::new(20)));

    let res = app.execute_contract(
        Addr::unchecked("user1"),
        contract.clone(),
        &ExecuteMsg::SetDistribution {
            distribution: distribution.clone(),
        },
        &vec![],
    );

    assert!(res.is_ok());

    let contract_distribution: Option<Distribution> = app
        .wrap()
        .query_wasm_smart(
            &contract,
            &QueryMsg::Distribution {
                addr: "user1".to_string(),
            },
        )
        .unwrap();
}
