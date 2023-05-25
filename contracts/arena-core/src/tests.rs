use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128, WasmMsg};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use dao_core::{query::GetItemResponse, state::ProposalModule};
use dao_interface::{Admin, ModuleInstantiateInfo};

use crate::msg::{InstantiateExt, InstantiateMsg};

const CREATOR: &str = "ismellike";

fn sudo_proposal_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_sudo::contract::execute,
        dao_proposal_sudo::contract::instantiate,
        dao_proposal_sudo::contract::query,
    );
    Box::new(contract)
}

#[test]
fn attach_arena_core() {
    let mut app = App::default();
    let gov_id = app.store_code(dao_testing::contracts::dao_core_contract());
    let govmod_id = app.store_code(sudo_proposal_contract());
    let dpm_id = app.store_code(arena_testing::contracts::dao_proposal_multiple_contract());
    let arena_id = app.store_code(arena_testing::contracts::arena_dao_core_contract());

    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR.to_owned(),
    };
    let gov_instantiate = dao_core::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "Arena DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: govmod_id,
            msg: to_binary(&govmod_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        }],
        initial_items: None,
    };

    let gov_addr = app
        .instantiate_contract(
            gov_id,
            Addr::unchecked(CREATOR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    let proposal_module = &app
        .wrap()
        .query_wasm_smart::<Vec<ProposalModule>>(
            gov_addr.clone(),
            &dao_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: Some(1u32),
            },
        )
        .unwrap()[0];

    let res = app.execute_contract(
        Addr::unchecked(CREATOR),
        proposal_module.address.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: gov_addr.to_string(),
                funds: vec![],
                msg: to_binary(&dao_core::msg::ExecuteMsg::UpdateProposalModules {
                    to_add: vec![ModuleInstantiateInfo {
                        code_id: dpm_id,
                        msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                            voting_strategy:
                                dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                    quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                                },
                            min_voting_period: None,
                            max_voting_period: cw_utils::Duration::Time(604800u64),
                            only_members_execute: false,
                            allow_revoting: false,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                    info: ModuleInstantiateInfo {
                                        code_id: arena_id,
                                        msg: to_binary(&InstantiateMsg {
                                            deposit_info: None,
                                            open_proposal_submission: false,
                                            extension: InstantiateExt {
                                                competition_modules_instantiate_info: vec![],
                                                rulesets: vec![],
                                                tax: Decimal::new(Uint128::from(
                                                    150000000000000u128,
                                                )),
                                            },
                                        })
                                        .unwrap(),
                                        admin: Some(Admin::CoreModule {}),
                                        label: "Arena Core".to_string(),
                                    },
                                },
                            close_proposal_on_execution_failure: true,
                        })
                        .unwrap(),
                        admin: Some(Admin::CoreModule {}),
                        label: "Proposal Multiple".to_string(),
                    }],
                    to_disable: vec![],
                })
                .unwrap(),
            }
            .into()],
        },
        &vec![],
    );

    assert!(res.is_ok());

    let item_response: GetItemResponse = app
        .wrap()
        .query_wasm_smart(
            gov_addr,
            &dao_core::msg::QueryMsg::GetItem {
                key: crate::contract::ITEM_KEY.to_owned(),
            },
        )
        .unwrap();

    assert!(item_response.item.is_some());
}
