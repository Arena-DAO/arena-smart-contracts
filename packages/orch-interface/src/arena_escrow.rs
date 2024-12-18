use arena_interface::escrow::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cw_orch::interface;
use cw_orch::prelude::*;

pub const CONTRACT_ID: &str = "arena_escrow";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct ArenaEscrowContract;

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
