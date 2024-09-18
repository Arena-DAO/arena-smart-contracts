use cw_orch::interface;
#[cfg(not(target_arch = "wasm32"))]
use cw_orch::prelude::*;

#[allow(unused_imports)]
use cw_abc::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub const CONTRACT_ID: &str = "cw_abc";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct CwAbcContract;

#[cfg(not(target_arch = "wasm32"))]
impl<Chain> Uploadable for CwAbcContract<Chain> {
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
                cw_abc::contract::execute,
                cw_abc::contract::instantiate,
                cw_abc::contract::query,
            )
            .with_reply(cw_abc::contract::reply),
        )
    }
}
