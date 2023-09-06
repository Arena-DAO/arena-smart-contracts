use cosmwasm_std::{to_binary, Addr, CosmosMsg, Deps, Empty, StdResult, Uint128, WasmMsg};
use cw4::{Member, MemberListResponse};
use cw_balance::MemberShare;
use dao_voting::multiple_choice::MultipleChoiceOptions;

use crate::msg::ExecuteBase;

pub fn get_competition_choices(
    deps: Deps,
    id: Uint128,
    competition_module: &Addr,
    cw4_group: &Addr,
) -> StdResult<MultipleChoiceOptions> {
    // Retrieve all team members from the CW4 group contract
    let teams = get_all_members(deps, cw4_group)?;

    // Create a multiple-choice option for each team, associating it with the corresponding competition result
    let mut options = teams
        .iter()
        .enumerate()
        .map(|(team_number, x)| {
            Ok(dao_voting::multiple_choice::MultipleChoiceOption {
                title: format!("Team {}", team_number + 1),
                description: "This team is the winner.".to_string(),
                msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: competition_module.to_string(),
                    msg: to_binary(&ExecuteBase::<Empty, Empty>::ProcessCompetition {
                        id,
                        distribution: Some(vec![MemberShare {
                            addr: x.addr.clone(),
                            shares: Uint128::one(),
                        }]),
                    })?,
                    funds: vec![],
                })],
            })
        })
        .collect::<StdResult<Vec<dao_voting::multiple_choice::MultipleChoiceOption>>>()?;

    // Add a 'Draw' option to handle cases where no team wins
    options.push(dao_voting::multiple_choice::MultipleChoiceOption {
        title: "Draw".to_string(),
        description: "No team won.".to_string(),
        msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: competition_module.to_string(),
            msg: to_binary(&ExecuteBase::<Empty, Empty>::ProcessCompetition {
                id,
                distribution: None,
            })?,
            funds: vec![],
        })],
    });

    // Create a proposal message for the competition result
    Ok(MultipleChoiceOptions { options })
}

fn get_all_members(deps: Deps, cw4_group_addr: &Addr) -> StdResult<Vec<Member>> {
    let mut all_members: Vec<Member> = vec![];
    let mut start_after = None;
    const LIMIT: u32 = 50;

    loop {
        let response: MemberListResponse = deps.querier.query_wasm_smart(
            cw4_group_addr,
            &cw4::Cw4QueryMsg::ListMembers {
                start_after: start_after.clone(),
                limit: Some(LIMIT),
            },
        )?;

        if response.members.is_empty() {
            break;
        }

        start_after = Some(response.members.last().unwrap().addr.clone());
        all_members.extend(response.members);
    }

    Ok(all_members)
}
