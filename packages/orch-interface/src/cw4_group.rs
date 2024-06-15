#[cfg(not(target_arch = "wasm32"))]
use cw_orch::environment::ChainInfoOwned;
use cw_orch::interface;
#[cfg(not(target_arch = "wasm32"))]
use cw_orch::prelude::*;

#[allow(unused_imports)]
use cosmwasm_std::Empty;
#[allow(unused_imports)]
use cw4_group::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

pub const CONTRACT_ID: &str = "cw4_group";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = CONTRACT_ID)]
pub struct Cw4Group;

#[cfg(not(target_arch = "wasm32"))]
impl<Chain> Uploadable for Cw4Group<Chain> {
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
                cw721_base::entry::execute,
                cw721_base::entry::instantiate,
                cw721_base::entry::query,
            )
            .with_migrate(cw721_base::entry::migrate),
        )
    }
}
