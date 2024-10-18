use cw_orch::prelude::*;

use orch_interface::{
    arena_competition_enrollment::ArenaCompetitionEnrollmentContract,
    arena_core::ArenaCoreContract, arena_escrow::ArenaEscrowContract,
    arena_group::ArenaGroupContract, arena_league_module::ArenaLeagueModuleContract,
    arena_payment_registry::ArenaPaymentRegistryContract,
    arena_token_gateway::ArenaTokenGatewayContract,
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
    pub arena_token_gateway: ArenaTokenGatewayContract<Chain>,
    pub arena_payment_registry: ArenaPaymentRegistryContract<Chain>,
    pub arena_group: ArenaGroupContract<Chain>,
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
            arena_token_gateway: ArenaTokenGatewayContract::new(chain.clone()),
            arena_payment_registry: ArenaPaymentRegistryContract::new(chain.clone()),
            arena_group: ArenaGroupContract::new(chain.clone()),
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
        self.arena_token_gateway.upload()?;
        self.arena_payment_registry.upload()?;
        self.arena_group.upload()?;

        if with_dao_dao {
            self.dao_dao.upload()?;
            self.cw4_group.upload()?;
        }

        Ok(())
    }
}
