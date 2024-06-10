use cw_orch::interface;
#[cfg(not(target_arch = "wasm32"))]
use cw_orch::prelude::*;

#[allow(unused_imports)]
use arena_escrow::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub const CONTRACT_ID: &str = "arena_escrow";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct ArenaEscrowContract;

#[cfg(not(target_arch = "wasm32"))]
impl<Chain> Uploadable for ArenaEscrowContract<Chain> {
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
                arena_escrow::contract::execute,
                arena_escrow::contract::instantiate,
                arena_escrow::contract::query,
            )
            .with_migrate(arena_escrow::contract::migrate),
        )
    }
}