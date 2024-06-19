use arena_tournament_module::state::EliminationType;
use cosmwasm_std::{ensure, Coin, DepsMut, Env, MessageInfo, Response, StdError, Uint128};
use cw_utils::{must_pay, Expiration};

use crate::{
    state::{CompetitionInfo, CompetitionType},
    ContractError,
};

pub fn create_competition(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_members: Option<Uint128>,
    max_members: Uint128,
    entry_fee: Option<Coin>,
    expiration: Expiration,
    category_id: Option<Uint128>,
    competition_info: CompetitionInfo<String>,
    is_creator_member: Option<bool>,
) -> Result<Response, ContractError> {
    ensure!(
        !expiration.is_expired(&env.block),
        ContractError::StdError(StdError::generic_err(
            "Cannot create an expired competition enrollment"
        ))
    );
    ensure!(
        !matches!(competition_info, CompetitionInfo::Existing(_)),
        ContractError::StdError(StdError::generic_err("Competition info cannot be existing"))
    );

    let min_min_members = Uint128::new(match &competition_info {
        CompetitionInfo::Pending {
            competition_type, ..
        } => match competition_type {
            CompetitionType::Wager {} => 2,
            CompetitionType::League { distribution, .. } => std::cmp::max(distribution.len(), 2),
            CompetitionType::Tournament {
                elimination_type,
                distribution,
            } => match elimination_type {
                EliminationType::SingleElimination {
                    play_third_place_match: _,
                } => std::cmp::max(4, distribution.len()),
                EliminationType::DoubleElimination => std::cmp::max(3, distribution.len()),
            },
        },
        CompetitionInfo::Existing(_) => unreachable!(),
    } as u128);
    if let Some(min_members) = min_members {
        ensure!(
            min_members < max_members,
            ContractError::StdError(StdError::generic_err(
                "Min members cannot be larger than max members"
            ))
        );
        ensure!(
            min_members > min_min_members,
            ContractError::StdError(StdError::generic_err(
                "Min members cannot be less than min members by competition ".to_string()
                    + &min_min_members.to_string()
            ))
        )
    }

    // Defaults
    let is_creator_member = is_creator_member.unwrap_or(false);

    if let Some(entry_fee) = entry_fee {
        if is_creator_member {
            let paid_amount = must_pay(&info, &entry_fee.denom)?;

            if paid_amount != entry_fee {
                // Return error
            }
        }
    }

    todo!()
}
