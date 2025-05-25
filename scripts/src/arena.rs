use cosmwasm_std::to_json_binary;
use cw_orch::prelude::*;

use cw_utils::Duration;
use dao_interface::state::{Admin, ModuleInstantiateInfo};
use dao_voting::threshold::Threshold;
use orch_interface::{
    arena_competition_enrollment::ArenaCompetitionEnrollmentContract,
    arena_core::ArenaCoreContract, arena_escrow::ArenaEscrowContract,
    arena_group::ArenaGroupContract, arena_league_module::ArenaLeagueModuleContract,
    arena_payment_registry::ArenaPaymentRegistryContract,
    arena_team_enrollments::ArenaTeamEnrollmentsContract,
    arena_tournament_module::ArenaTournamentModuleContract,
    arena_wager_module::ArenaWagerModuleContract, cw4_group::Cw4Group,
};

use crate::dao_dao::DaoDao;

pub struct Arena<Chain> {
    pub arena_core: ArenaCoreContract<Chain>,
    pub arena_wager_module: ArenaWagerModuleContract<Chain>,
    pub arena_league_module: ArenaLeagueModuleContract<Chain>,
    pub arena_tournament_module: ArenaTournamentModuleContract<Chain>,
    pub arena_escrow: ArenaEscrowContract<Chain>,
    pub arena_competition_enrollment: ArenaCompetitionEnrollmentContract<Chain>,
    pub arena_payment_registry: ArenaPaymentRegistryContract<Chain>,
    pub arena_group: ArenaGroupContract<Chain>,
    pub arena_team_enrollments: ArenaTeamEnrollmentsContract<Chain>,
    pub dao_dao: DaoDao<Chain>,
    pub cw4_group: Cw4Group<Chain>,
}

impl<Chain: CwEnv> Arena<Chain> {
    pub fn new(chain: Chain) -> Arena<Chain> {
        Arena::<Chain> {
            arena_core: ArenaCoreContract::new(chain.clone()),
            arena_wager_module: ArenaWagerModuleContract::new(chain.clone()),
            arena_league_module: ArenaLeagueModuleContract::new(chain.clone()),
            arena_tournament_module: ArenaTournamentModuleContract::new(chain.clone()),
            arena_escrow: ArenaEscrowContract::new(chain.clone()),
            arena_competition_enrollment: ArenaCompetitionEnrollmentContract::new(chain.clone()),
            arena_payment_registry: ArenaPaymentRegistryContract::new(chain.clone()),
            arena_group: ArenaGroupContract::new(chain.clone()),
            arena_team_enrollments: ArenaTeamEnrollmentsContract::new(chain.clone()),
            dao_dao: DaoDao::new(chain.clone()),
            cw4_group: Cw4Group::new(chain.clone()),
        }
    }

    pub fn upload(&self, with_dao_dao: bool) -> Result<(), CwOrchError> {
        self.arena_escrow.upload()?;
        self.arena_core.upload()?;
        self.arena_wager_module.upload()?;
        self.arena_league_module.upload()?;
        self.arena_tournament_module.upload()?;
        self.arena_competition_enrollment.upload()?;
        self.arena_payment_registry.upload()?;
        self.arena_group.upload()?;
        self.arena_team_enrollments.upload()?;

        if with_dao_dao {
            self.dao_dao.upload()?;
            self.cw4_group.upload()?;
        }

        Ok(())
    }

    /// Deploy a new DAO DAO with specified members
    ///
    /// # Arguments
    ///
    /// * `primary_address` - The address of the primary member (used to register)
    /// * `additional_members` - Vector of additional member addresses to include in the DAO
    ///
    /// # Returns
    ///
    /// * Result containing the contract addresses of the deployed DAO components
    pub fn deploy_dao(
        &self,
        primary_address: &Addr,
        additional_members: Vec<&Addr>,
    ) -> Result<Addr, CwOrchError> {
        let original_dao_address = self.dao_dao.dao_core.address()?;

        // Create a vector with all members, starting with admin
        let mut all_members = vec![cw4::Member {
            addr: primary_address.to_string(),
            weight: 1,
        }];

        // Add additional members with equal voting weight
        for member in additional_members {
            all_members.push(cw4::Member {
                addr: member.to_string(),
                weight: 1,
            });
        }

        // Prepare CW4 voting module info
        let voting_module_info = ModuleInstantiateInfo {
            code_id: self.dao_dao.dao_voting_cw4.code_id()?,
            msg: to_json_binary(&dao_voting_cw4::msg::InstantiateMsg {
                group_contract: dao_voting_cw4::msg::GroupContract::New {
                    cw4_group_code_id: self.cw4_group.code_id()?,
                    cw4_group_salt: None,
                    initial_members: all_members,
                },
            })?,
            admin: Some(Admin::CoreModule {}),
            label: "cw4 voting module".to_string(),
            funds: None,
            salt: None,
        };

        // Prepare a simple proposal module without prepropose or extensions
        let proposal_modules = vec![ModuleInstantiateInfo {
            code_id: self.dao_dao.dao_proposal_single.code_id()?,
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
                delegation_module: None,
            })?,
            admin: Some(Admin::CoreModule {}),
            funds: None,
            salt: None,
            label: "DAO Proposal Single".to_string(),
        }];

        // Instantiate DAO core
        self.dao_dao.dao_core.instantiate(
            &dao_interface::msg::InstantiateMsg {
                dao_uri: None,
                admin: None,
                name: "Member DAO".to_string(),
                description: "A DAO with multiple members".to_string(),
                image_url: None,
                automatically_add_cw20s: true,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: voting_module_info,
                proposal_modules_instantiate_info: proposal_modules,
                initial_items: None,
                initial_actions: None,
            },
            None,
            &[],
        )?;

        let result = self.dao_dao.dao_core.address()?;

        self.dao_dao.dao_core.set_address(&original_dao_address);

        Ok(result)
    }
}
