use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty};
use cw_orch::prelude::*;
use cw_utils::Duration;
use dao_interface::{
    state::{Admin, ModuleInstantiateInfo},
    CoreQueryMsgFns as _,
};
use dao_voting::threshold::Threshold;
use std::collections::BTreeMap;

use arena_interface::{
    core::{InstantiateExt, InstantiateMsg, NewCompetitionCategory, QueryExtFns as _},
    fees::TaxConfiguration,
};

use crate::Arena;

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for Arena<Chain> {
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let arena = Arena::new(chain.clone());
        arena.upload(true)?;
        Ok(arena)
    }

    fn deploy_on(chain: Chain, admin: Addr) -> Result<Self, Self::Error> {
        let arena = Self::store_on(chain)?;

        // Prepare voting module info
        let voting_module_info = ModuleInstantiateInfo {
            code_id: arena.dao_dao.dao_proposal_sudo.code_id()?,
            msg: to_json_binary(&dao_proposal_sudo::msg::InstantiateMsg {
                root: admin.to_string(),
            })?,
            admin: Some(Admin::CoreModule {}),
            label: "sudo voting module".to_string(),
            funds: vec![],
        };

        // Instantiate payment registry
        arena.arena_payment_registry.instantiate(
            &arena_interface::registry::InstantiateMsg {},
            None,
            None,
        )?;

        // Prepare proposal modules
        let proposal_modules = vec![
            ModuleInstantiateInfo {
                code_id: arena.dao_dao.dao_proposal_sudo.code_id()?,
                msg: to_json_binary(&dao_proposal_sudo::msg::InstantiateMsg {
                    root: admin.to_string(),
                })?,
                admin: Some(Admin::CoreModule {}),
                label: "sudo proposal module".to_string(),
                funds: vec![],
            },
            ModuleInstantiateInfo {
                code_id: arena.dao_dao.dao_proposal_single.code_id()?,
                msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                    threshold: Threshold::AbsolutePercentage {
                        percentage: dao_voting::threshold::PercentageThreshold::Majority {},
                    },
                    max_voting_period: Duration::Height(10u64),
                    min_voting_period: None,
                    only_members_execute: false,
                    allow_revoting: true,
                    pre_propose_info: dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                        info: ModuleInstantiateInfo {
                            code_id: arena.arena_core.code_id()?,
                            msg: to_json_binary(&InstantiateMsg {
                                deposit_info: None,
                                submission_policy:
                                    dao_voting::pre_propose::PreProposeSubmissionPolicy::Specific {
                                        dao_members: true,
                                        allowlist: vec![],
                                        denylist: vec![],
                                    },
                                extension: InstantiateExt {
                                    competition_modules_instantiate_info: Some(vec![
                                        dao_interface::state::ModuleInstantiateInfo {
                                            code_id: arena.arena_tournament_module.code_id()?,
                                            msg: to_json_binary(
                                                &arena_tournament_module::msg::InstantiateMsg {
                                                    key: "Tournaments".to_string(),
                                                    description: "Knockout competitions"
                                                        .to_string(),
                                                    extension: Empty {},
                                                },
                                            )?,
                                            admin: Some(dao_interface::state::Admin::CoreModule {}),
                                            label: "Tournament Module".to_string(),
                                            funds: vec![],
                                        },
                                        dao_interface::state::ModuleInstantiateInfo {
                                            code_id: arena.arena_wager_module.code_id()?,
                                            msg: to_json_binary(
                                                &arena_wager_module::msg::InstantiateMsg {
                                                    key: "Wagers".to_string(),
                                                    description: "Skill-based wagers".to_string(),
                                                    extension: Empty {},
                                                },
                                            )?,
                                            admin: Some(dao_interface::state::Admin::CoreModule {}),
                                            label: "Wager Module".to_string(),
                                            funds: vec![],
                                        },
                                        dao_interface::state::ModuleInstantiateInfo {
                                            code_id: arena.arena_league_module.code_id()?,
                                            msg: to_json_binary(
                                                &arena_league_module::msg::InstantiateMsg {
                                                    key: "Leagues".to_string(),
                                                    description: "Round-robin tournaments"
                                                        .to_string(),
                                                    extension: Empty {},
                                                },
                                            )?,
                                            admin: Some(dao_interface::state::Admin::CoreModule {}),
                                            label: "League Module".to_string(),
                                            funds: vec![],
                                        },
                                    ]),
                                    rulesets: None,
                                    categories: Some(vec![
                                        NewCompetitionCategory {
                                            name: "Category".to_string(),
                                        },
                                        NewCompetitionCategory {
                                            name: "Other Category".to_string(),
                                        },
                                    ]),
                                    tax: Decimal::from_ratio(5u128, 100u128),
                                    tax_configuration: TaxConfiguration {
                                        cw20_msg: None,
                                        cw721_msg: None,
                                    },
                                    rating_period: Duration::Time(604800u64),
                                    payment_registry: Some(
                                        arena.arena_payment_registry.addr_str()?,
                                    ),
                                },
                            })?,
                            admin: Some(Admin::CoreModule {}),
                            funds: vec![],
                            label: "Arena Core".to_string(),
                        },
                    },
                    close_proposal_on_execution_failure: true,
                    veto: None,
                })?,
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "Arena Core Proposal".to_string(),
            },
            ModuleInstantiateInfo {
                code_id: arena.dao_dao.dao_proposal_single.code_id()?,
                msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                    threshold: Threshold::AbsolutePercentage {
                        percentage: dao_voting::threshold::PercentageThreshold::Majority {},
                    },
                    max_voting_period: Duration::Height(10),
                    min_voting_period: None,
                    only_members_execute: false,
                    allow_revoting: false,
                    pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
                    close_proposal_on_execution_failure: true,
                    veto: None,
                })?,
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "DAO Proposal Single".to_string(),
            },
        ];

        // Instantiate DAO
        arena.dao_dao.dao_core.instantiate(
            &dao_interface::msg::InstantiateMsg {
                dao_uri: None,
                admin: None,
                name: "Arena DAO".to_string(),
                description: "The next iteration of competition infrastructure".to_string(),
                image_url: None,
                automatically_add_cw20s: true,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: voting_module_info,
                proposal_modules_instantiate_info: proposal_modules,
                initial_items: None,
                initial_dao_actions: None,
            },
            None,
            None,
        )?;

        // Configuration
        let proposal_modules = arena.dao_dao.dao_core.proposal_modules(None, None)?;
        for proposal_module in proposal_modules {
            match proposal_module.prefix.as_str() {
                "A" => arena
                    .dao_dao
                    .dao_proposal_sudo
                    .set_address(&proposal_module.address),
                "C" => arena
                    .dao_dao
                    .dao_proposal_single
                    .set_address(&proposal_module.address),
                &_ => {}
            };
        }

        let get_item_response = arena.dao_dao.dao_core.get_item("Arena".to_string())?;
        arena
            .arena_core
            .set_address(&Addr::unchecked(get_item_response.item.unwrap()));

        // Set the competition modules
        let competition_modules = arena.arena_core.competition_modules(None, None, None)?;

        let competition_module_map = competition_modules
            .into_iter()
            .map(|x| (x.key, x.addr))
            .collect::<BTreeMap<String, Addr>>();
        arena
            .arena_tournament_module
            .set_address(competition_module_map.get("Tournaments").unwrap());
        arena
            .arena_wager_module
            .set_address(competition_module_map.get("Wagers").unwrap());
        arena
            .arena_league_module
            .set_address(competition_module_map.get("Leagues").unwrap());

        // Instantiate the enrollment module
        arena.arena_competition_enrollment.instantiate(
            &arena_competition_enrollment::msg::InstantiateMsg {
                owner: arena.arena_core.addr_str()?,
            },
            Some(&arena.dao_dao.dao_core.address()?),
            None,
        )?;

        Ok(arena)
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let arena = Self::new(chain);
        Ok(arena)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(
        &mut self,
    ) -> Vec<Box<&mut dyn cw_orch::prelude::ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.dao_dao.dao_core),
            Box::new(&mut self.dao_dao.dao_proposal_single),
            Box::new(&mut self.dao_dao.dao_proposal_sudo),
            Box::new(&mut self.arena_core),
            Box::new(&mut self.arena_tournament_module),
            Box::new(&mut self.arena_wager_module),
            Box::new(&mut self.arena_league_module),
            Box::new(&mut self.arena_escrow),
            Box::new(&mut self.arena_competition_enrollment),
            Box::new(&mut self.arena_token_gateway),
            Box::new(&mut self.arena_payment_registry),
        ]
    }
}
