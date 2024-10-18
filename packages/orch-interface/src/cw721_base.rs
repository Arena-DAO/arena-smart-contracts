use cw_orch::environment::ChainInfoOwned;
use cw_orch::interface;
use cw_orch::prelude::*;

use cosmwasm_std::Empty;
use cw721_base::msg::{ExecuteMsg as GenExecuteMsg, InstantiateMsg, QueryMsg as GenQueryMsg};
use cw721_base::Extension;

pub type ExecuteMsg = GenExecuteMsg<Extension, Empty>;
pub type QueryMsg = GenQueryMsg<Empty>;

pub const CONTRACT_ID: &str = "cw721_base";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = CONTRACT_ID)]
pub struct Cw721Base;

impl<Chain> Uploadable for Cw721Base<Chain> {
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
