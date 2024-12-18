use arena_interface::core::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cw_orch::interface;
use cw_orch::prelude::*;

pub const CONTRACT_ID: &str = "arena_core";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct ArenaCoreContract;

impl<Chain> Uploadable for ArenaCoreContract<Chain> {
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
                arena_core::contract::execute,
                arena_core::contract::instantiate,
                arena_core::contract::query,
            )
            .with_migrate(arena_core::contract::migrate)
            .with_reply(arena_core::contract::reply),
        )
    }
}
