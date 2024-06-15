use cw_orch::prelude::*;

use orch_interface::{
    arena_core::ArenaCoreContract, arena_escrow::ArenaEscrowContract,
    arena_tournament_module::ArenaTournamentModuleContract,
};

use crate::dao_dao::DaoDao;

pub struct Arena<Chain> {
    pub arena_core: ArenaCoreContract<Chain>,
    pub arena_tournament_module: ArenaTournamentModuleContract<Chain>,
    pub arena_escrow: ArenaEscrowContract<Chain>,
    pub dao_dao: DaoDao<Chain>,
}

impl<Chain: CwEnv> Arena<Chain> {
    pub fn new(chain: Chain) -> Arena<Chain> {
        Arena::<Chain> {
            arena_core: ArenaCoreContract::new(chain.clone()),
            arena_tournament_module: ArenaTournamentModuleContract::new(chain.clone()),
            arena_escrow: ArenaEscrowContract::new(chain.clone()),
            dao_dao: DaoDao::new(chain.clone()),
        }
    }

    pub fn upload(&self, with_dao_dao: bool) -> Result<(), CwOrchError> {
        self.arena_core.upload()?;
        self.arena_tournament_module.upload()?;
        self.arena_escrow.upload()?;

        if with_dao_dao {
            self.dao_dao.upload()?;
        }

        Ok(())
    }
}
