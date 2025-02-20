use arena_interface::competition::types::APIProcessing;
use cosmwasm_std::{ensure_eq, DepsMut, MessageInfo, Response, StdError, Uint128};
use cw_competition_base::error::CompetitionError;

use crate::contract::CompetitionModule;

pub fn process_competition_api(
    deps: DepsMut,
    info: MessageInfo,
    competition_id: Uint128,
    result: serde_json::Value,
) -> Result<Response, CompetitionError> {
    let competition_module = CompetitionModule::default();

    let competition = competition_module
        .competitions
        .load(deps.storage, competition_id.u128())?;

    let api_processing = competition
        .extension
        .api_processing
        .ok_or(StdError::generic_err(
            "API processing is not configured for this competition",
        ))?;

    match api_processing {
        APIProcessing::Yunite {
            guild_id: _,
            tournament_id: _,
            avs,
        } => {
            ensure_eq!(info.sender, avs, CompetitionError::Unauthorized {});

            let _ = result; // Process the json result into a distribution
        }
    };

    Ok(Response::new().add_attribute("action", "process_competition_api"))
}
