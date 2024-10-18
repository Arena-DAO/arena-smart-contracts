use cw4_group::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw_orch::environment::ChainInfoOwned;
use cw_orch::interface;
use cw_orch::prelude::*;

pub const CONTRACT_ID: &str = "cw4_group";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, cosmwasm_std::Empty, id = CONTRACT_ID)]
pub struct Cw4Group;

impl<Chain> Uploadable for Cw4Group<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path(CONTRACT_ID)
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            cw4_group::contract::execute,
            cw4_group::contract::instantiate,
            cw4_group::contract::query,
        ))
    }
}
