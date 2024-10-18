use arena_competition_enrollment::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cw_orch::interface;
use cw_orch::prelude::*;

pub const CONTRACT_ID: &str = "arena_competition_enrollment";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct ArenaCompetitionEnrollmentContract;

impl<Chain> Uploadable for ArenaCompetitionEnrollmentContract<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path(CONTRACT_ID)
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(
                arena_competition_enrollment::contract::execute,
                arena_competition_enrollment::contract::instantiate,
                arena_competition_enrollment::contract::query,
            )
            .with_migrate(arena_competition_enrollment::contract::migrate)
            .with_reply(arena_competition_enrollment::contract::reply),
        )
    }
}
