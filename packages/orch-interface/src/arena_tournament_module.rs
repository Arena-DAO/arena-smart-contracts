use arena_tournament_module::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cw_orch::interface;
use cw_orch::prelude::*;

pub const CONTRACT_ID: &str = "arena_tournament_module";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct ArenaTournamentModuleContract;

impl<Chain> Uploadable for ArenaTournamentModuleContract<Chain> {
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
                arena_tournament_module::contract::execute,
                arena_tournament_module::contract::instantiate,
                arena_tournament_module::contract::query,
            )
            .with_migrate(arena_tournament_module::contract::migrate)
            .with_reply(arena_tournament_module::contract::reply),
        )
    }
}
