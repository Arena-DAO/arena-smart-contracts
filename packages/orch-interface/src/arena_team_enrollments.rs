use arena_team_enrollments::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw_orch::interface;
use cw_orch::prelude::*;

pub const CONTRACT_ID: &str = "arena_team_enrollments";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty, id = CONTRACT_ID)]
pub struct ArenaTeamEnrollmentsContract;

impl<Chain> Uploadable for ArenaTeamEnrollmentsContract<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path(CONTRACT_ID)
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            arena_team_enrollments::contract::execute,
            arena_team_enrollments::contract::instantiate,
            arena_team_enrollments::contract::query,
        ))
    }
}
