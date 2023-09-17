use std::marker::PhantomData;

use arena_core_interface::msg::ProposalDetails;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw_balance::MemberShare;
use cw_competition::{
    escrow::CompetitionEscrowDistributeMsg,
    msg::{ExecuteBase, InstantiateBase, QueryBase},
    proposal::get_competition_choices,
    state::{Competition, CompetitionResponse, CompetitionStatus, Config},
};
use cw_ownable::{get_ownership, initialize_owner};
use cw_storage_plus::{Bound, Item, Map};
use cw_utils::parse_reply_instantiate_data;
use dao_interface::state::ProposalModule;
use dao_voting::pre_propose::ProposalCreationPolicy;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CompetitionError;

pub const DAO_REPLY_ID: u64 = 1;
pub const ESCROW_REPLY_ID: u64 = 2;
pub const PROCESS_REPLY_ID: u64 = 3;
pub const PROPOSALS_REPLY_ID: u64 = 4;

pub struct CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt> {
    pub config: Item<'static, Config>,
    pub competition_count: Item<'static, Uint128>,
    pub competitions: Map<'static, u128, Competition<CompetitionExt>>,
    pub escrows: Map<'static, Addr, u128>,
    pub temp_competition: Item<'static, u128>,

    instantiate_type: PhantomData<InstantiateExt>,
    execute_type: PhantomData<ExecuteExt>,
    query_type: PhantomData<QueryExt>,
}

impl<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
    CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
{
    const fn new(
        config_key: &'static str,
        competition_count_key: &'static str,
        competitions_key: &'static str,
        escrows_key: &'static str,
        temp_competition_key: &'static str,
    ) -> Self {
        Self {
            config: Item::new(config_key),
            competition_count: Item::new(competition_count_key),
            competitions: Map::new(competitions_key),
            escrows: Map::new(escrows_key),
            temp_competition: Item::new(temp_competition_key),
            instantiate_type: PhantomData,
            execute_type: PhantomData,
            query_type: PhantomData,
        }
    }
}

impl<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt> Default
    for CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
{
    fn default() -> Self {
        Self::new(
            "config",
            "competition_count",
            "competitions",
            "escrows",
            "temp_competition",
        )
    }
}

impl<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
    CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
where
    CompetitionExt: Serialize + DeserializeOwned,
    QueryExt: JsonSchema,
{
    pub fn instantiate(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        msg: InstantiateBase<InstantiateExt>,
    ) -> StdResult<Response> {
        self.config.save(
            deps.storage,
            &Config {
                key: msg.key.clone(),
                description: msg.description.clone(),
            },
        )?;
        let ownership = initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;
        self.competition_count
            .save(deps.storage, &Uint128::zero())?;

        Ok(Response::new()
            .add_attribute("key", msg.key)
            .add_attribute("description", msg.description)
            .add_attributes(ownership.into_attributes()))
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteBase<ExecuteExt, CompetitionExt>,
    ) -> Result<Response, CompetitionError> {
        match msg {
            ExecuteBase::JailCompetition {
                id,
                proposal_details,
            } => self.execute_jail_competition(
                deps,
                env,
                id,
                proposal_details.title,
                proposal_details.description,
            ),
            ExecuteBase::CreateCompetition {
                competition_dao,
                escrow,
                name,
                description,
                expiration,
                rules,
                ruleset,
                extension,
            } => self.execute_create_competition(
                deps,
                env,
                competition_dao,
                escrow,
                name,
                description,
                expiration,
                rules,
                ruleset,
                extension,
            ),
            ExecuteBase::ProcessCompetition { id, distribution } => {
                self.execute_process_competition(deps, info, id, distribution)
            }
            ExecuteBase::GenerateProposals {
                id,
                proposal_module_addr,
                proposal_details,
            } => self.execute_generate_proposals(
                deps,
                env,
                proposal_module_addr,
                id,
                proposal_details.title,
                proposal_details.description,
            ),
            ExecuteBase::UpdateOwnership(action) => {
                let ownership =
                    cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
                Ok(Response::new().add_attributes(ownership.into_attributes()))
            }
            ExecuteBase::Activate {} => self.execute_activate(deps, info),
            ExecuteBase::Extension { .. } => Ok(Response::default()),
        }
    }

    pub fn execute_activate(
        &self,
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response, CompetitionError> {
        let id = self.escrows.may_load(deps.storage, info.sender.clone())?;

        if id.is_none() {
            return Err(CompetitionError::UnknownEscrow {
                addr: info.sender.to_string(),
            });
        }

        let id = id.unwrap();

        let competition = self.competitions.may_load(deps.storage, id)?;

        if competition.is_none() {
            return Err(CompetitionError::UnknownCompetitionId { id });
        }

        let mut competition = competition.unwrap();

        competition.status = CompetitionStatus::Active;

        self.competitions.save(deps.storage, id, &competition)?;

        Ok(Response::new()
            .add_attribute("id", id.to_string())
            .add_attribute("action", "activate")
            .add_attribute("escrow", info.sender))
    }

    pub fn execute_generate_proposals(
        &self,
        deps: DepsMut,
        env: Env,
        proposal_module_addr: String,
        id: Uint128,
        title: String,
        description: String,
    ) -> Result<Response, CompetitionError> {
        let competition = self.competitions.may_load(deps.storage, id.u128())?;

        if competition.is_none() {
            return Err(CompetitionError::UnknownCompetitionId { id: id.u128() });
        }

        let competition = competition.unwrap();
        let proposal_module_addr = deps.api.addr_validate(&proposal_module_addr)?;
        let voting_module: Addr = deps.querier.query_wasm_smart(
            competition.dao.clone(),
            &dao_interface::msg::QueryMsg::VotingModule {},
        )?;
        let cw4_group: Addr = deps.querier.query_wasm_smart(
            voting_module,
            &dao_voting_cw4::msg::QueryMsg::GroupContract {},
        )?;

        // Ensure that the proposal module is active on the competition's DAO
        let proposal_modules: Vec<ProposalModule> = deps.querier.query_wasm_smart(
            competition.dao.clone(),
            &dao_interface::msg::QueryMsg::ActiveProposalModules {
                start_after: None,
                limit: None,
            },
        )?;

        if !proposal_modules
            .iter()
            .any(|x| x.address == proposal_module_addr)
        {
            return Err(CompetitionError::StdError(StdError::GenericErr {
                msg: "The given proposal module was not active on the competition's DAO"
                    .to_string(),
            }));
        }

        // Ensure that the proposal module is a dao-proposal-multiple
        let info = cw2::query_contract_info(&deps.querier, &proposal_module_addr)?;

        if !info.contract.contains("dao-proposal-multiple") {
            return Err(CompetitionError::StdError(StdError::GenericErr {
                msg: "The given proposal module is not of type dao-proposal-multiple".to_string(),
            }));
        }

        let creation_policy: ProposalCreationPolicy = deps.querier.query_wasm_smart(
            proposal_module_addr.clone(),
            &dao_proposal_multiple::msg::QueryMsg::ProposalCreationPolicy {},
        )?;
        if !creation_policy.is_permitted(&env.contract.address) {
            return Err(CompetitionError::StdError(StdError::GenericErr {
                msg: "The competition module cannot propose on the given proposal module"
                    .to_string(),
            }));
        }

        let choices =
            get_competition_choices(deps.as_ref(), id, &env.contract.address, &cw4_group)?;
        let sub_msg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: proposal_module_addr.to_string(),
                msg: to_binary(&dao_proposal_multiple::msg::ExecuteMsg::Propose {
                    title,
                    description,
                    choices,
                    proposer: None,
                })?,
                funds: vec![],
            }),
            PROPOSALS_REPLY_ID,
        );
        self.temp_competition.save(deps.storage, &id.u128())?;

        Ok(Response::new()
            .add_attribute("action", "generate_proposals")
            .add_attribute("id", id)
            .add_attribute("proposal_module", proposal_module_addr.to_string())
            .add_submessage(sub_msg))
    }

    pub fn execute_jail_competition(
        &self,
        deps: DepsMut,
        env: Env,
        id: Uint128,
        title: String,
        description: String,
    ) -> Result<Response, CompetitionError> {
        let dao = get_ownership(deps.storage)?;

        if dao.owner.is_none() {
            return Err(CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NoOwner,
            ));
        }

        self.competitions.update(
            deps.storage,
            id.u128(),
            |x| -> Result<_, CompetitionError> {
                if x.is_none() {
                    return Err(CompetitionError::UnknownCompetitionId { id: id.u128() });
                }
                let mut competition = x.unwrap();

                if competition.status != CompetitionStatus::Active {
                    return Err(CompetitionError::InvalidCompetitionStatus {
                        current_status: competition.status,
                    });
                }
                if !competition.expiration.is_expired(&env.block) {
                    return Err(CompetitionError::CompetitionNotExpired {});
                }

                competition.status = CompetitionStatus::Jailed;

                Ok(competition)
            },
        )?;

        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: dao.owner.unwrap().to_string(),
            msg: to_binary(&arena_core_interface::msg::ExecuteMsg::Extension {
                msg: arena_core_interface::msg::ExecuteExt::Jail {
                    id,
                    proposal_details: ProposalDetails { title, description },
                },
            })?,
            funds: vec![],
        });

        Ok(Response::new()
            .add_attribute("action", "jail_wager")
            .add_attribute("id", id)
            .add_message(msg))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_create_competition(
        &self,
        deps: DepsMut,
        env: Env,
        competition_dao: dao_interface::state::ModuleInstantiateInfo,
        escrow: dao_interface::state::ModuleInstantiateInfo,
        name: String,
        description: String,
        expiration: cw_utils::Expiration,
        rules: Vec<String>,
        ruleset: Option<Uint128>,
        extension: CompetitionExt,
    ) -> Result<Response, CompetitionError> {
        let id = self
            .competition_count
            .update(deps.storage, |x| -> StdResult<_> {
                Ok(x.checked_add(Uint128::one())?)
            })?;
        let competition = Competition {
            start_height: env.block.height,
            id,
            dao: Addr::unchecked("temp"),
            escrow: Addr::unchecked("temp"),
            name,
            description,
            expiration,
            rules,
            ruleset,
            status: CompetitionStatus::Pending,
            extension,
            has_generated_proposals: false,
        };
        let dao = self.get_dao(deps.as_ref())?;
        let msgs = vec![
            SubMsg::reply_always(competition_dao.into_wasm_msg(dao.clone()), DAO_REPLY_ID),
            SubMsg::reply_always(escrow.into_wasm_msg(dao), ESCROW_REPLY_ID),
        ];
        self.competitions
            .save(deps.storage, id.u128(), &competition)?;
        self.temp_competition.save(deps.storage, &id.u128())?;

        Ok(Response::new()
            .add_attribute("action", "create_competition")
            .add_attribute("id", id)
            .add_submessages(msgs))
    }

    pub fn execute_process_competition(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: Uint128,
        distribution: Option<Vec<cw_balance::MemberShare>>,
    ) -> Result<Response, CompetitionError> {
        let competition = match self.competitions.may_load(deps.storage, id.u128())? {
            Some(val) => Ok(val),
            None => Err(CompetitionError::UnknownCompetitionId { id: id.u128() }),
        }?;
        let dao = self.get_dao(deps.as_ref())?;

        match competition.status {
            CompetitionStatus::Active => {
                if competition.dao != info.sender && dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
                Ok(())
            }
            CompetitionStatus::Jailed => {
                if dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
                Ok(())
            }
            _ => Err(CompetitionError::InvalidCompetitionStatus {
                current_status: competition.status,
            }),
        }?;

        // Apply tax
        let distribution = match distribution {
            Some(mut member_shares) => {
                let arena_core = cw_ownable::get_ownership(deps.storage)?.owner.unwrap();
                let tax: Decimal = deps.querier.query_wasm_smart(
                    arena_core,
                    &arena_core_interface::msg::QueryMsg::QueryExtension {
                        msg: arena_core_interface::msg::QueryExt::Tax {
                            height: Some(competition.start_height),
                        },
                    },
                )?;

                if !tax.is_zero() {
                    let precision_multiplier = Uint128::from(10000u128);
                    let sum = member_shares
                        .iter()
                        .try_fold(Uint128::zero(), |accumulator, x| {
                            accumulator.checked_add(x.shares)
                        })?;

                    let dao_shares = tax
                        .checked_mul(Decimal::from_atomics(sum, 0u32)?)?
                        .checked_div(Decimal::one().checked_sub(tax)?)?;
                    let dao_shares = dao_shares
                        .checked_mul(Decimal::from_atomics(precision_multiplier, 0u32)?)?
                        .checked_div(Decimal::from_atomics(
                            Uint128::new(10u128).checked_pow(dao_shares.decimal_places())?,
                            0u32,
                        )?)?
                        .atomics();

                    for member in &mut member_shares {
                        member.shares = member.shares.checked_mul(precision_multiplier)?;
                    }

                    member_shares.push(MemberShare {
                        addr: dao.to_string(),
                        shares: dao_shares,
                    });
                }

                Some(member_shares)
            }
            None => None,
        };

        let sub_msg = SubMsg::reply_on_success(
            CompetitionEscrowDistributeMsg {
                distribution,
                remainder_addr: dao.to_string(),
            }
            .into_cosmos_msg(competition.escrow)?,
            PROCESS_REPLY_ID,
        );

        Ok(Response::new()
            .add_attribute("action", "process_competition")
            .add_submessage(sub_msg))
    }

    pub fn query(
        &self,
        deps: Deps,
        env: Env,
        msg: QueryBase<QueryExt, CompetitionExt>,
    ) -> StdResult<Binary> {
        match msg {
            QueryBase::Config {} => to_binary(&self.config.load(deps.storage)?),
            QueryBase::Competition {
                id,
                include_ruleset,
            } => to_binary(
                &self
                    .competitions
                    .load(deps.storage, id.u128())?
                    .to_response(deps, &env.block, include_ruleset)?,
            ),
            QueryBase::Competitions {
                start_after,
                limit,
                include_ruleset,
            } => to_binary(&self.query_competitions(
                deps,
                env,
                start_after,
                limit,
                include_ruleset,
            )?),
            QueryBase::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
            QueryBase::CompetitionCount {} => {
                to_binary(&self.competition_count.load(deps.storage)?)
            }
            QueryBase::QueryExtension { .. } => Ok(Binary::default()),
            QueryBase::_Phantom(_) => Ok(Binary::default()),
        }
    }

    fn get_dao(&self, deps: Deps) -> Result<Addr, cw_ownable::OwnershipError> {
        let core = cw_ownable::get_ownership(deps.storage)?;
        if core.owner.is_none() {
            return Err(cw_ownable::OwnershipError::NoOwner);
        }
        Ok(deps.querier.query_wasm_smart(
            core.owner.unwrap(),
            &dao_pre_propose_base::msg::QueryMsg::<Empty>::Dao {},
        )?)
    }

    fn query_competitions(
        &self,
        deps: Deps,
        env: Env,
        start_after: Option<Uint128>,
        limit: Option<u32>,
        include_ruleset: Option<bool>,
    ) -> StdResult<Vec<CompetitionResponse<CompetitionExt>>> {
        let start_after_bound = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(10).max(30);

        cw_paginate::paginate_map(
            &self.competitions,
            deps.storage,
            start_after_bound,
            Some(limit),
            |_x, y| y.to_response(deps, &env.block, include_ruleset),
        )
    }

    pub fn reply(
        &self,
        deps: DepsMut,
        _env: Env,
        msg: Reply,
    ) -> Result<Response, CompetitionError> {
        match msg.id {
            DAO_REPLY_ID => self.reply_dao(deps, msg),
            ESCROW_REPLY_ID => self.reply_escrow(deps, msg),
            PROCESS_REPLY_ID => self.reply_process(deps, msg),
            PROPOSALS_REPLY_ID => self.reply_proposals(deps),
            _ => Err(CompetitionError::UnknownReplyId { id: msg.id }),
        }
    }

    pub fn reply_dao(&self, deps: DepsMut, msg: Reply) -> Result<Response, CompetitionError> {
        let result = parse_reply_instantiate_data(msg)?;
        let addr = deps.api.addr_validate(&result.contract_address)?;
        let id = self.temp_competition.load(deps.storage)?;

        self.competitions
            .update(deps.storage, id, |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        competition.dao = addr.clone();
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;

        Ok(Response::new()
            .add_attribute("action", "reply_dao")
            .add_attribute("dao_addr", addr))
    }

    pub fn reply_proposals(&self, deps: DepsMut) -> Result<Response, CompetitionError> {
        let id = self.temp_competition.load(deps.storage)?;

        self.competitions
            .update(deps.storage, id, |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        competition.has_generated_proposals = true;

                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;

        Ok(Response::new().add_attribute("action", "reply_proposals"))
    }

    pub fn reply_escrow(&self, deps: DepsMut, msg: Reply) -> Result<Response, CompetitionError> {
        let result = parse_reply_instantiate_data(msg)?;
        let addr = deps.api.addr_validate(&result.contract_address)?;
        let id = self.temp_competition.load(deps.storage)?;

        self.competitions
            .update(deps.storage, id, |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        competition.escrow = addr.clone();
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;
        self.escrows.save(deps.storage, addr.clone(), &id)?;

        Ok(Response::new()
            .add_attribute("action", "reply_escrow")
            .add_attribute("escrow_addr", addr))
    }

    pub fn reply_process(&self, deps: DepsMut, _msg: Reply) -> Result<Response, CompetitionError> {
        let id = self.temp_competition.load(deps.storage)?;

        self.competitions
            .update(deps.storage, id, |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        competition.status = CompetitionStatus::Inactive {};
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;

        Ok(Response::new().add_attribute("action", "reply_process"))
    }
}
