use arena_interface::registry::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use cw_orch::interface;
use cw_orch::prelude::*;

pub const CONTRACT_ID: &str = "arena_payment_registry";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct ArenaPaymentRegistryContract;

impl<Chain> Uploadable for ArenaPaymentRegistryContract<Chain> {
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
                arena_payment_registry::contract::execute,
                arena_payment_registry::contract::instantiate,
                arena_payment_registry::contract::query,
            )
            .with_migrate(arena_payment_registry::contract::migrate),
        )
    }
}
