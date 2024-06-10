use arena_core_interface::{
    fees::TaxConfiguration,
    msg::{InstantiateExt, InstantiateMsg, QueryExtFns},
};
use cosmwasm_std::{to_json_binary, Decimal};
use cw_orch::prelude::*;
use dao_interface::{
    state::{Admin, ModuleInstantiateInfo},
    CoreQueryMsgFns,
};

use dao_voting::threshold::Threshold;

use crate::arena::Arena;

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for Arena<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let arena = Arena::new(chain.clone());

        arena.upload(true)?;

        Ok(arena)
    }

    fn deploy_on(chain: Chain, admin: Addr) -> Result<Self, CwOrchError> {
        // ########### Upload ##############
        let arena = Self::store_on(chain)?;

        // ########### Instantiate ##############
        let sudo_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
            root: admin.to_string(),
        };
        arena.dao_dao.dao_core.instantiate(
            &dao_interface::msg::InstantiateMsg {
                dao_uri: None,
                admin: None,
                name: "Arena DAO".to_string(),
                description: "The next iteration of competition infrastructure".to_string(),
                image_url: None,
                automatically_add_cw20s: true,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: ModuleInstantiateInfo {
                    code_id: arena.dao_dao.dao_proposal_sudo.code_id()?,
                    msg: to_json_binary(&sudo_instantiate)?,
                    admin: Some(Admin::CoreModule {}),
                    label: "voting module".to_string(),
                    funds: vec![],
                },
                proposal_modules_instantiate_info: vec![
                    ModuleInstantiateInfo {
                        code_id: arena.dao_dao.dao_proposal_sudo.code_id()?,
                        msg: to_json_binary(&sudo_instantiate)?,
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
                            max_voting_period: cw_utils::Duration::Height(10u64), // prod will be time (~3 days?)
                            min_voting_period: None,
                            only_members_execute: false, // prod will be true
                            allow_revoting: true,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                    info: ModuleInstantiateInfo {
                                        code_id: arena.arena_core.code_id()?,
                                        msg: to_json_binary(&InstantiateMsg {
                                            deposit_info: None,
                                            open_proposal_submission: false,
                                            extension: InstantiateExt {
                                                competition_modules_instantiate_info: vec![
                                                    dao_interface_master::state::ModuleInstantiateInfo {
                                                        code_id: arena.arena_tournament_module.code_id()?,
                                                        msg: to_json_binary(
                                                            &arena_tournament_module::msg::InstantiateMsg {
                                                                key: "tournaments".to_string(),
                                                                description: "Knockout competitions".to_string(),
                                                                extension: Empty {},
                                                            },
                                                        )?,
                                                        admin: Some(
                                                            dao_interface_master::state::Admin::CoreModule {},
                                                        ),
                                                        label: "Tournament Module".to_string(),
                                                    },
                                                ],
                                                rulesets: vec![],
                                                categories: vec![],
                                                tax: Decimal::from_ratio(5u128, 100u128),
                                                tax_configuration: TaxConfiguration {
                                                    cw20_msg: None,
                                                    cw721_msg: None,
                                                },
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
                        label: "proposal module".to_string(),
                    },
                ],
                initial_items: None,
            },
            None,
            None,
        )?;

        let proposal_modules = arena.dao_dao.dao_core.proposal_modules(None, None)?;
        arena
            .dao_dao
            .dao_proposal_sudo
            .set_address(&proposal_modules[0].address);
        arena
            .dao_dao
            .dao_proposal_single
            .set_address(&proposal_modules[1].address);

        let get_item_response = arena.dao_dao.dao_core.get_item("Arena".to_string())?;
        arena
            .arena_core
            .set_address(&Addr::unchecked(get_item_response.item.unwrap()));

        let competition_modules = arena.arena_core.competition_modules(None, None, None)?;
        arena
            .arena_tournament_module
            .set_address(&Addr::unchecked(competition_modules[0].addr.clone()));

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
            Box::new(&mut self.arena_escrow),
        ]
    }
}