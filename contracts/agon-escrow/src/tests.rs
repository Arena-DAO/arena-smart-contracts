use cosmwasm_std::{testing::mock_info, Addr, Binary, Coin, Empty, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_competition::{CwCompetitionResultMsg, CwCompetitionStateChangedMsg};
use cw_disbursement::MemberBalance;
use cw_disbursement::MemberShare;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_tokens::GenericTokenBalance;

use crate::{
    contract::{self, instantiate},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

fn mock_app() -> App {
    App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(ADDR1),
                vec![Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128::from(BEGINNING_BALANCE),
                }],
            )
            .unwrap();
    })
}

fn instantiate_cw20(app: &mut App, code_id: u64, msg: cw20_base::msg::InstantiateMsg) -> Addr {
    app.instantiate_contract(code_id, Addr::unchecked(ADDR1), &msg, &[], "cw20", None)
        .unwrap()
}

fn instantiate_escrow(app: &mut App, code_id: u64, sender: Addr, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(code_id, sender.clone(), &msg, &[], "escrow", None)
        .unwrap()
}

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn contract_escrow() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        contract::execute,
        instantiate,
        contract::query,
    ))
}

fn create_context(app: &mut App) -> Context {
    let cw20_id = app.store_code(contract_cw20());
    let escrow_id = app.store_code(contract_escrow());

    let token = instantiate_cw20(
        app,
        cw20_id,
        cw20_base::msg::InstantiateMsg {
            name: String::from("Agon"),
            symbol: String::from("AGON"),
            decimals: 6,
            initial_balances: vec![{
                Cw20Coin {
                    address: String::from(ADDR1),
                    amount: Uint128::from(BEGINNING_BALANCE),
                }
            }],
            mint: None,
            marketing: None,
        },
    );

    let escrow = instantiate_escrow(
        app,
        escrow_id,
        Addr::unchecked(ADDR1),
        InstantiateMsg {
            due: vec![MemberBalance {
                member: ADDR1.to_string(),
                balances: vec![GenericTokenBalance {
                    addr: None,
                    denom: Some(DENOM.to_string()),
                    amount: Uint128::from(500u128),
                    token_type: cw_tokens::GenericTokenType::Native,
                }],
            }],
            stake: vec![],
            arbiter: None,
        },
    );

    Context { token, escrow }
}

pub const DENOM: &str = "AgonN";
pub const ADDR1: &str = "member-1";
pub const _ADDR2: &str = "member-2";
pub const _ADDR3: &str = "member-3";
pub const _ADDR4: &str = "member-4";
pub const BEGINNING_BALANCE: u128 = 1000u128;

struct Context {
    pub token: Addr,
    pub escrow: Addr,
}

#[test]
fn tests() {
    let mut app = mock_app();
    let context = create_context(&mut app);

    let due: Vec<GenericTokenBalance> = app
        .wrap()
        .query_wasm_smart(
            context.escrow.clone(),
            &QueryMsg::Due {
                member: ADDR1.to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        due,
        vec![GenericTokenBalance {
            addr: None,
            denom: Some(DENOM.to_string()),
            amount: Uint128::from(500u128),
            token_type: cw_tokens::GenericTokenType::Native,
        }]
    );

    let msg = Cw20ExecuteMsg::Send {
        contract: context.escrow.to_string(),
        amount: due.first().unwrap().amount,
        msg: Binary::default(),
    };
    let info = mock_info(ADDR1, &[]);
    app.execute_contract(
        info.sender.clone(),
        context.token.clone(),
        &msg,
        &info.funds,
    )
    .unwrap();

    let due: Vec<GenericTokenBalance> = app
        .wrap()
        .query_wasm_smart(
            context.escrow.clone(),
            &QueryMsg::Due {
                member: ADDR1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(due, vec![]);
    let balance: Vec<GenericTokenBalance> = app
        .wrap()
        .query_wasm_smart(
            context.escrow.clone(),
            &QueryMsg::Balance {
                member: ADDR1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        balance,
        vec![GenericTokenBalance {
            addr: Some(context.token.clone()),
            denom: None,
            amount: Uint128::from(500u128),
            token_type: cw_tokens::GenericTokenType::Cw20,
        }]
    );
    let total_balance: Vec<GenericTokenBalance> = app
        .wrap()
        .query_wasm_smart(context.escrow.clone(), &&QueryMsg::Total {})
        .unwrap();
    assert_eq!(balance, total_balance);

    let msg = ExecuteMsg::HandleCompetitionStateChanged(CwCompetitionStateChangedMsg {
        old_state: cw_competition::CompetitionState::Pending,
        new_state: cw_competition::CompetitionState::Active,
    });
    app.execute_contract(
        info.sender.clone(),
        context.escrow.clone(),
        &msg,
        &info.funds,
    )
    .unwrap();

    let msg = ExecuteMsg::HandleCompetitionResult(CwCompetitionResultMsg {
        distribution: Some(vec![MemberShare {
            addr: ADDR1.to_string(),
            shares: Uint128::one(),
        }]),
    });
    app.execute_contract(
        info.sender.clone(),
        context.escrow.clone(),
        &msg,
        &info.funds,
    )
    .unwrap();
}
