use std::marker::PhantomData;

use arena_core_interface::msg::{ProposeMessage, ProposeMessages};
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw_balance::MemberShare;
use cw_competition::{
    escrow::CompetitionEscrowDistributeMsg,
    msg::{ExecuteBase, InstantiateBase, QueryBase},
    state::{Competition, CompetitionResponse, CompetitionStatus, Config},
};
use cw_ownable::{get_ownership, initialize_owner};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use cw_utils::parse_reply_instantiate_data;
use dao_interface::state::{ModuleInstantiateInfo, ProposalModule};
use dao_voting::{pre_propose::ProposalCreationPolicy, proposal::SingleChoiceProposeMsg};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CompetitionError;

pub const DAO_REPLY_ID: u64 = 1;
pub const ESCROW_REPLY_ID: u64 = 2;
pub const PROCESS_REPLY_ID: u64 = 3;
pub const PROPOSALS_REPLY_ID: u64 = 4;
pub const PRECISION_MULTIPLIER: u128 = 100_000;

pub struct CompetitionIndexes<'a, CompetitionExt> {
    pub status: MultiIndex<'a, String, Competition<CompetitionExt>, u128>,
}

impl<'a, CompetitionExt: Serialize + Clone + DeserializeOwned>
    IndexList<Competition<CompetitionExt>> for CompetitionIndexes<'a, CompetitionExt>
{
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<Competition<CompetitionExt>>> + '_> {
        let v: Vec<&dyn Index<Competition<CompetitionExt>>> = vec![&self.status];
        Box::new(v.into_iter())
    }
}

pub struct CompetitionModuleContract<
    InstantiateExt,
    ExecuteExt,
    QueryExt,
    CompetitionExt: Serialize + Clone + DeserializeOwned,
> {
    pub config: Item<'static, Config>,
    pub competition_count: Item<'static, Uint128>,
    pub competitions: IndexedMap<
        'static,
        u128,
        Competition<CompetitionExt>,
        CompetitionIndexes<'static, CompetitionExt>,
    >,
    pub escrow_to_competition_map: Map<'static, Addr, u128>,
    pub temp_competition: Item<'static, u128>,

    instantiate_type: PhantomData<InstantiateExt>,
    execute_type: PhantomData<ExecuteExt>,
    query_type: PhantomData<QueryExt>,
}

impl<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
    > CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
{
    const fn new(
        config_key: &'static str,
        competition_count_key: &'static str,
        competitions_key: &'static str,
        competitions_status_key: &'static str,
        escrows_key: &'static str,
        temp_competition_key: &'static str,
    ) -> Self {
        Self {
            config: Item::new(config_key),
            competition_count: Item::new(competition_count_key),
            competitions: Self::competitions(competitions_key, competitions_status_key),
            escrow_to_competition_map: Map::new(escrows_key),
            temp_competition: Item::new(temp_competition_key),
            instantiate_type: PhantomData,
            execute_type: PhantomData,
            query_type: PhantomData,
        }
    }

    const fn competitions(
        competitions_key: &'static str,
        competitions_status_key: &'static str,
    ) -> IndexedMap<
        'static,
        u128,
        Competition<CompetitionExt>,
        CompetitionIndexes<'static, CompetitionExt>,
    > {
        let indexes = CompetitionIndexes {
            status: MultiIndex::new(
                |_x, d: &Competition<CompetitionExt>| d.status.to_string(),
                competitions_key,
                competitions_status_key,
            ),
        };
        IndexedMap::new(competitions_key, indexes)
    }
}

impl<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
    > Default for CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
{
    fn default() -> Self {
        Self::new(
            "config",
            "competition_count",
            "competitions",
            "competitions__status",
            "escrows",
            "temp_competition",
        )
    }
}

impl<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
    > CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
where
    CompetitionExt: Serialize + DeserializeOwned,
    QueryExt: JsonSchema,
{
    pub fn instantiate(
        &self,
        deps: DepsMut,
        env: Env,
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
            .add_attribute("competition_module_addr", env.contract.address)
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
            ExecuteBase::JailCompetition { propose_message } => {
                self.execute_jail_competition(deps, env, info, propose_message)
            }
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
            ExecuteBase::DeclareResult { propose_message } => {
                self.execute_declare_result(deps, env, info, propose_message)
            }
            ExecuteBase::ProcessCompetition { id, distribution } => {
                self.execute_process_competition(deps, info, id, distribution)
            }
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
        let id = self
            .escrow_to_competition_map
            .may_load(deps.storage, info.sender.clone())?;

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

    pub fn execute_declare_result(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        propose_message: ProposeMessage,
    ) -> Result<Response, CompetitionError> {
        let id = propose_message.id;
        let competition = self.competitions.may_load(deps.storage, id.u128())?;
        if competition.is_none() {
            return Err(CompetitionError::UnknownCompetitionId { id: id.u128() });
        }
        let competition = competition.unwrap();

        // Find a valid proposal module
        let proposal_modules: Vec<ProposalModule> = deps.querier.query_wasm_smart(
            competition.dao.clone(),
            &dao_interface::msg::QueryMsg::ActiveProposalModules {
                start_after: None,
                limit: None,
            },
        )?;

        let mut proposal_module_addr = None;
        let mut proposer = None;
        for proposal_module in proposal_modules {
            // Ensure that the proposal module is of type dao-proposal-single
            let contract_info_result =
                cw2::query_contract_info(&deps.querier, &proposal_module.address);

            if contract_info_result.is_err() {
                continue;
            }
            if !contract_info_result
                .unwrap()
                .contract
                .contains("dao-proposal-single")
            {
                continue;
            }

            let creation_policy_result = deps.querier.query_wasm_smart::<ProposalCreationPolicy>(
                proposal_module.address.clone(),
                &dao_proposal_single::msg::QueryMsg::ProposalCreationPolicy {},
            );
            if creation_policy_result.is_err() {
                continue;
            }
            let creation_policy = creation_policy_result.unwrap();
            if !creation_policy.is_permitted(&env.contract.address) {
                continue;
            }

            proposal_module_addr = Some(proposal_module.address);
            proposer = match creation_policy {
                ProposalCreationPolicy::Anyone {} => None,
                ProposalCreationPolicy::Module { addr: _ } => Some(info.sender.to_string()),
            }
        }
        if proposal_module_addr.is_none() {
            return Err(CompetitionError::StdError(StdError::GenericErr {
                msg: "Could not find an accessible dao-proposal-single module".to_owned(),
            }));
        }

        // Construct message
        let propose_message = ProposeMessages::Propose(SingleChoiceProposeMsg {
            title: propose_message.title,
            description: propose_message.description,
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteBase::<Empty, Empty>::ProcessCompetition {
                    id: propose_message.id,
                    distribution: propose_message.distribution,
                })?,
                funds: vec![],
            })],
            proposer,
        });

        // Prepare reply
        let sub_msg = SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: proposal_module_addr.as_ref().unwrap().to_string(),
                msg: to_binary(&propose_message)?,
                funds: vec![],
            }),
            PROPOSALS_REPLY_ID,
        );
        self.temp_competition.save(deps.storage, &id.u128())?;

        Ok(Response::new()
            .add_attribute("action", "generate_proposals")
            .add_attribute("id", id)
            .add_attribute("proposal_module", proposal_module_addr.unwrap().to_string())
            .add_submessage(sub_msg))
    }

    pub fn execute_jail_competition(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        propose_message: ProposeMessage,
    ) -> Result<Response, CompetitionError> {
        let dao = get_ownership(deps.storage)?;

        if dao.owner.is_none() {
            return Err(CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NoOwner,
            ));
        }

        let id = propose_message.id;
        self.competitions.update(
            deps.storage,
            id.u128(),
            |x| -> Result<_, CompetitionError> {
                if x.is_none() {
                    return Err(CompetitionError::UnknownCompetitionId { id: id.u128() });
                }
                let mut competition = x.unwrap();

                if competition.status != CompetitionStatus::Jailed {
                    if competition.status != CompetitionStatus::Active {
                        return Err(CompetitionError::InvalidCompetitionStatus {
                            current_status: competition.status,
                        });
                    }
                    if !competition.expiration.is_expired(&env.block) {
                        return Err(CompetitionError::CompetitionNotExpired {});
                    }
                }

                // Check that the user is a member of the competition DAO
                if info.sender != competition.admin_dao {
                    let voting_power_response: dao_interface::voting::VotingPowerAtHeightResponse =
                        deps.querier.query_wasm_smart(
                            competition.dao.clone(),
                            &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                                address: info.sender.to_string(),
                                height: None,
                            },
                        )?;
                    if voting_power_response.power.is_zero() {
                        return Err(CompetitionError::Unauthorized {});
                    }
                }

                competition.status = CompetitionStatus::Jailed;
                Ok(competition)
            },
        )?;

        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: dao.owner.unwrap().to_string(),
            msg: to_binary(&arena_core_interface::msg::ExecuteMsg::Propose {
                msg: propose_message,
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
        competition_dao: ModuleInstantiateInfo,
        escrow: Option<ModuleInstantiateInfo>,
        name: String,
        description: String,
        expiration: cw_utils::Expiration,
        rules: Vec<String>,
        ruleset: Option<Uint128>,
        extension: CompetitionExt,
    ) -> Result<Response, CompetitionError> {
        if expiration.is_expired(&env.block) {
            return Err(CompetitionError::StdError(StdError::GenericErr {
                msg: "Cannot create an expired competition".to_string(),
            }));
        }

        let id = self
            .competition_count
            .update(deps.storage, |x| -> StdResult<_> {
                Ok(x.checked_add(Uint128::one())?)
            })?;
        let admin_dao = self.get_dao(deps.as_ref())?;
        let competition = Competition {
            admin_dao: admin_dao.clone(),
            start_height: env.block.height,
            id,
            dao: env.contract.address.clone(),
            escrow: None,
            name,
            description,
            expiration,
            rules,
            ruleset,
            status: CompetitionStatus::Pending,
            extension,
            has_generated_proposals: false,
            result: None,
        };
        let mut msgs = vec![SubMsg::reply_on_success(
            competition_dao.into_wasm_msg(admin_dao.clone()),
            DAO_REPLY_ID,
        )];
        if let Some(escrow) = escrow {
            msgs.push(SubMsg::reply_on_success(
                escrow.into_wasm_msg(admin_dao),
                ESCROW_REPLY_ID,
            ));
        }
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
        distribution: Option<Vec<cw_balance::MemberShare<String>>>,
    ) -> Result<Response, CompetitionError> {
        // Load related data
        let mut competition = match self.competitions.may_load(deps.storage, id.u128())? {
            Some(val) => Ok(val),
            None => Err(CompetitionError::UnknownCompetitionId { id: id.u128() }),
        }?;

        // Validate status
        match competition.status {
            CompetitionStatus::Active => {
                if competition.dao != info.sender && competition.admin_dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
                Ok(())
            }
            CompetitionStatus::Jailed => {
                if competition.admin_dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
                Ok(())
            }
            _ => Err(CompetitionError::InvalidCompetitionStatus {
                current_status: competition.status.clone(),
            }),
        }?;

        // Update result
        competition.result = distribution
            .clone()
            .map(|x| x.iter().map(|y| y.to_validated(deps.as_ref())).collect())
            .transpose()?;
        self.competitions
            .save(deps.storage, id.u128(), &competition)?;

        // Perform escrow actions if applicable
        let response = Response::new().add_attribute("action", "process_competition");
        if let Some(escrow) = competition.escrow {
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
                        let precision_multiplier = Uint128::from(PRECISION_MULTIPLIER);
                        let sum = member_shares
                            .iter()
                            .try_fold(Uint128::zero(), |accumulator, x| {
                                accumulator.checked_add(x.shares)
                            })?;

                        let dao_shares = tax
                            .checked_mul(Decimal::from_atomics(
                                sum.checked_mul(precision_multiplier)?,
                                0u32,
                            )?)?
                            .checked_div(Decimal::one().checked_sub(tax)?)?;
                        let dao_shares = dao_shares
                            .checked_div(Decimal::from_atomics(
                                Uint128::new(10u128).checked_pow(dao_shares.decimal_places())?,
                                0u32,
                            )?)?
                            .atomics();

                        for member in &mut member_shares {
                            member.shares = member.shares.checked_mul(precision_multiplier)?;
                        }

                        member_shares.push(MemberShare {
                            addr: competition.admin_dao.to_string(),
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
                    remainder_addr: competition.admin_dao.to_string(),
                }
                .into_cosmos_msg(escrow)?,
                PROCESS_REPLY_ID,
            );

            Ok(response.add_submessage(sub_msg))
        } else {
            Ok(response)
        }
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
                status,
            } => to_binary(&self.query_competitions(
                deps,
                env,
                start_after,
                limit,
                include_ruleset,
                status,
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
        status: Option<CompetitionStatus>,
    ) -> StdResult<Vec<CompetitionResponse<CompetitionExt>>> {
        let start_after_bound = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(10).max(30);

        match status {
            None => cw_paginate::paginate_indexed_map(
                &self.competitions,
                deps.storage,
                start_after_bound,
                Some(limit),
                |_x, y| y.to_response(deps, &env.block, include_ruleset),
            ),
            Some(status) => self
                .competitions
                .idx
                .status
                .prefix(status.to_string())
                .range(
                    deps.storage,
                    start_after_bound,
                    None,
                    cosmwasm_std::Order::Ascending,
                )
                .map(|x| x.and_then(|y| y.1.to_response(deps, &env.block, include_ruleset)))
                .take(limit as usize)
                .collect::<StdResult<Vec<_>>>(),
        }
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
                        competition.escrow = Some(addr.clone());
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;
        self.escrow_to_competition_map
            .save(deps.storage, addr.clone(), &id)?;

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
