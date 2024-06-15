use cw_orch::interface;
#[cfg(not(target_arch = "wasm32"))]
use cw_orch::prelude::*;

#[allow(unused_imports)]
use arena_wager_module::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub const CONTRACT_ID: &str = "arena_wager_module";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct ArenaWagerModuleContract;

#[cfg(not(target_arch = "wasm32"))]
impl<Chain> Uploadable for ArenaWagerModuleContract<Chain> {
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
                arena_wager_module::contract::execute,
                arena_wager_module::contract::instantiate,
                arena_wager_module::contract::query,
            )
            .with_migrate(arena_wager_module::contract::migrate)
            .with_reply(arena_wager_module::contract::reply),
        )
    }
}
