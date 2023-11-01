use std::ops::Add;

use arena_core_interface::msg::CompetitionModuleResponse;
use cosmwasm_std::{
    from_binary, to_binary, Addr, CosmosMsg, DepsMut, Empty, Env, OverflowError, OverflowOperation,
    Response, StdError, StdResult, Uint128, Uint64, WasmMsg,
};
use cw_competition_base::error::CompetitionError;
use cw_utils::Duration;
use dao_interface::state::ModuleInstantiateInfo;

use crate::{
    contract::CompetitionModule,
    state::{Match, Round, WAGERS_KEY},
    ContractError,
};

#[allow(clippy::too_many_arguments)]
pub fn instantiate_rounds(
    deps: DepsMut,
    env: Env,
    response: Response,
    teams: Vec<String>,
    round_duration: Duration,
    rules: Vec<String>,
    rulesets: Vec<Uint128>,
    wager_dao: ModuleInstantiateInfo,
    competition_name: String,
) -> Result<Response, ContractError> {
    // Convert team names to addresses
    let team_addresses: Vec<Addr> = teams
        .iter()
        .map(|name| deps.api.addr_validate(name))
        .collect::<StdResult<_>>()?;
    let team_count = team_addresses.len();

    // Calculate the number of rounds
    let rounds_count = if team_count % 2 == 0 {
        team_count - 1
    } else {
        team_count
    };
    let matches_per_round = (rounds_count + 1) / 2;

    // Generate match pairings for rounds
    let mut team_indexes: Vec<usize> = (1..=rounds_count + 1).collect();
    let mut rounds: Vec<Vec<(usize, usize)>> = Vec::new();
    for _ in 0..rounds_count {
        let round_pairings: Vec<(usize, usize)> = (0..matches_per_round)
            .filter_map(|m| {
                let idx1 = team_indexes[m];
                let idx2 = team_indexes[team_indexes.len() - 1 - m];
                if idx1 < team_count && idx2 < team_count {
                    Some((idx1, idx2))
                } else {
                    None
                }
            })
            .collect();
        rounds.push(round_pairings);
        team_indexes.rotate_right(1);
    }

    // Retrieve the current league ID
    let league_id = CompetitionModule::default()
        .competition_count
        .load(deps.storage)?;

    // Retreive the wager module
    let wager_key = WAGERS_KEY.load(deps.storage)?;
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    if ownership.owner.is_none() {
        return Err(ContractError::CompetitionError(
            CompetitionError::OwnershipError(cw_ownable::OwnershipError::NoOwner),
        ));
    }
    let arena_core = ownership.owner.unwrap();
    let wager_module: Option<CompetitionModuleResponse<String>> = deps.querier.query_wasm_smart(
        arena_core,
        &arena_core_interface::msg::QueryMsg::QueryExtension {
            msg: arena_core_interface::msg::QueryExt::CompetitionModule {
                query: arena_core_interface::msg::CompetitionModuleQuery::Key(
                    wager_key.clone(),
                    env.block.height,
                ),
            },
        },
    )?;

    if wager_module.is_none() || !wager_module.as_ref().unwrap().is_enabled {
        return Err(ContractError::CompetitionModuleNotAvailable { key: wager_key });
    }
    let wager_module = wager_module.unwrap();
    deps.api.addr_validate(&wager_module.addr)?;

    // Query wager id
    let mut wager_id: Uint128 = deps.querier.query_wasm_smart(
        &wager_module.addr,
        &cw_competition::msg::QueryBase::CompetitionCount::<Empty, Empty> {},
    )?;

    // Save rounds and matches to storage
    let mut msgs = vec![];
    let mut duration = round_duration;
    let mut match_number = 0u128;
    for (i, round_pairings) in rounds.iter().enumerate() {
        let round_number = i as u64;
        let mut matches = vec![];
        let expiration = duration.after(&env.block);

        for &(idx1, idx2) in round_pairings {
            matches.push(Match {
                team_1: team_addresses[idx1].clone(),
                team_2: team_addresses[idx2].clone(),
                result: None,
                wager_id,
                match_number: Uint128::from(match_number),
            });
            match_number += 1;
            wager_id = wager_id.checked_add(Uint128::one())?;

            // Alter dao data to set teams as members
            let mut competition_dao_msg: dao_interface::msg::InstantiateMsg =
                from_binary(&wager_dao.msg)?;
            let mut voting_module_instantiate_info_msg: dao_voting_cw4::msg::InstantiateMsg =
                from_binary(&competition_dao_msg.voting_module_instantiate_info.msg)?;

            voting_module_instantiate_info_msg.initial_members = vec![
                cw4::Member {
                    addr: team_addresses[idx1].to_string(),
                    weight: 1u64,
                },
                cw4::Member {
                    addr: team_addresses[idx2].to_string(),
                    weight: 1u64,
                },
            ];

            competition_dao_msg.voting_module_instantiate_info = ModuleInstantiateInfo {
                msg: to_binary(&voting_module_instantiate_info_msg)?,
                ..competition_dao_msg.voting_module_instantiate_info
            };

            let competition_dao = ModuleInstantiateInfo {
                msg: to_binary(&competition_dao_msg)?,
                ..wager_dao.clone()
            };

            // Prepare message
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: wager_module.addr.to_string(),
                msg: to_binary(
                    &cw_competition::msg::ExecuteBase::<Empty, Empty>::CreateCompetition {
                        competition_dao,
                        escrow: None,
                        name: format!("{} Wager Round {}", competition_name, i),
                        description: format!(
                            "This is a match of round {} from the league '{}'.",
                            i, competition_name
                        ),
                        expiration,
                        rules: rules.clone(),
                        rulesets: rulesets.clone(),
                        instantiate_extension: Empty {},
                    },
                )?,
                funds: vec![],
            }));
        }

        crate::state::rounds().save(
            deps.storage,
            (league_id.u128(), round_number),
            &Round {
                round_number: Uint64::from(round_number),
                matches,
                expiration,
            },
        )?;
        duration = duration.add(round_duration)?;
    }

    // Update competition rounds count
    let competition = CompetitionModule::default().competitions.update(
        deps.storage,
        league_id.u128(),
        |maybe_competition| {
            if let Some(mut competition) = maybe_competition {
                competition.extension.rounds = Uint64::from(rounds_count as u64);
                Ok(competition)
            } else {
                Err(StdError::NotFound {
                    kind: "Competition".to_string(),
                })
            }
        },
    )?;

    // Check competition expiration is greater than the last match's expiration + 1 match expiration duration
    let competition_expiration = duration.after(&env.block);
    if competition.expiration < competition_expiration {
        return Err(ContractError::OverflowError(OverflowError::new(
            OverflowOperation::Add,
            competition_expiration,
            competition.expiration,
        )));
    }

    Ok(response
        .add_attribute("round_duration", round_duration.to_string())
        .add_attribute("rounds", rounds_count.to_string())
        .add_messages(msgs))
}
