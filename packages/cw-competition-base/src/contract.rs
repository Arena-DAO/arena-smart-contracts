use std::marker::PhantomData;

use arena_core_interface::msg::{CompetitionModuleResponse, ProposeMessage, ProposeMessages};
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw_balance::MemberShare;
use cw_competition::{
    escrow::CompetitionEscrowDistributeMsg,
    msg::{ExecuteBase, HookDirection, InstantiateBase, QueryBase},
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
    CompetitionInstantiateExt: Into<CompetitionExt>,
> {
    pub config: Item<'static, Config>,
    pub competition_count: Item<'static, Uint128>,
    pub competitions: IndexedMap<
        'static,
        u128,
        Competition<CompetitionExt>,
        CompetitionIndexes<'static, CompetitionExt>,
    >,
    pub escrows_to_competitions: Map<'static, Addr, u128>,
    pub temp_competition: Item<'static, u128>,
    pub competition_hooks: Map<'static, (u128, Addr), HookDirection>,

    instantiate_type: PhantomData<InstantiateExt>,
    execute_type: PhantomData<ExecuteExt>,
    query_type: PhantomData<QueryExt>,
    competition_instantiate_type: PhantomData<CompetitionInstantiateExt>,
}

impl<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
        CompetitionInstantiateExt: Into<CompetitionExt>,
    >
    CompetitionModuleContract<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt,
        CompetitionInstantiateExt,
    >
{
    const fn new(
        config_key: &'static str,
        competition_count_key: &'static str,
        competitions_key: &'static str,
        competitions_status_key: &'static str,
        escrows_to_competitions_key: &'static str,
        temp_competition_key: &'static str,
        competition_hooks_key: &'static str,
    ) -> Self {
        Self {
            config: Item::new(config_key),
            competition_count: Item::new(competition_count_key),
            competitions: Self::competitions(competitions_key, competitions_status_key),
            escrows_to_competitions: Map::new(escrows_to_competitions_key),
            temp_competition: Item::new(temp_competition_key),
            competition_hooks: Map::new(competition_hooks_key),
            instantiate_type: PhantomData,
            execute_type: PhantomData,
            query_type: PhantomData,
            competition_instantiate_type: PhantomData,
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
        CompetitionInstantiateExt: Into<CompetitionExt>,
    > Default
    for CompetitionModuleContract<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt,
        CompetitionInstantiateExt,
    >
{
    fn default() -> Self {
        Self::new(
            "config",
            "competition_count",
            "competitions",
            "competitions__status",
            "escrows",
            "temp_competition",
            "competition_hooks",
        )
    }
}

impl<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
        CompetitionInstantiateExt: Into<CompetitionExt>,
    >
    CompetitionModuleContract<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt,
        CompetitionInstantiateExt,
    >
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
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteBase<ExecuteExt, CompetitionInstantiateExt>,
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
                rulesets,
                instantiate_extension,
            } => self.execute_create_competition(
                &mut deps,
                &env,
                competition_dao,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                instantiate_extension,
            ),
            ExecuteBase::ProposeResult { propose_message } => {
                self.execute_propose_result(deps, env, info, propose_message)
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
            ExecuteBase::AddCompetitionHook { id } => {
                self.execute_add_competition_hook(deps, info, id)
            }
            ExecuteBase::RemoveCompetitionHook { id } => {
                self.execute_remove_competition_hook(deps, info, id)
            }
            ExecuteBase::ExecuteCompetitionHook {
                id: _,
                distribution: _,
            }
            | ExecuteBase::Extension { .. } => Ok(Response::default()),
        }
    }

    pub fn validate_execute_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: Uint128,
    ) -> Result<(), CompetitionError> {
        // Validate hook
        if HookDirection::Incoming
            != self
                .competition_hooks
                .load(deps.storage, (id.u128(), info.sender.clone()))?
        {
            return Err(CompetitionError::Unauthorized {});
        }

        Ok(())
    }

    pub fn execute_add_competition_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: Uint128,
    ) -> Result<Response, CompetitionError> {
        // Load competition using the ID
        if !self.competitions.has(deps.storage, id.u128()) {
            return Err(CompetitionError::UnknownCompetitionId { id: id.u128() });
        };

        // Assert sender is a registered, active competition module
        let ownership = get_ownership(deps.storage)?;

        if ownership.owner.is_none() {
            return Err(CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NoOwner,
            ));
        }

        let competition_module: CompetitionModuleResponse<String> = deps.querier.query_wasm_smart(
            ownership.owner.unwrap(),
            &arena_core_interface::msg::QueryMsg::QueryExtension {
                msg: arena_core_interface::msg::QueryExt::CompetitionModule {
                    query: arena_core_interface::msg::CompetitionModuleQuery::Addr(
                        info.sender.to_string(),
                    ),
                },
            },
        )?;

        if !competition_module.is_enabled {
            return Err(CompetitionError::StdError(StdError::GenericErr {
                msg: "Competition module is not enabled".to_string(),
            }));
        }

        // Add competition hook
        self.competition_hooks.save(
            deps.storage,
            (id.u128(), info.sender.clone()),
            &HookDirection::Outgoing,
        )?;

        Ok(Response::new()
            .add_attribute("action", "add_competition_hook")
            .add_attribute("competition_module", info.sender)
            .add_attribute("id", id.to_string()))
    }

    pub fn execute_remove_competition_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: Uint128,
    ) -> Result<Response, CompetitionError> {
        // Load competition using the ID
        if !self.competitions.has(deps.storage, id.u128()) {
            return Err(CompetitionError::UnknownCompetitionId { id: id.u128() });
        };

        // Remove competition hook
        self.competition_hooks
            .remove(deps.storage, (id.u128(), info.sender.clone()));

        Ok(Response::new()
            .add_attribute("action", "add_competition_hook")
            .add_attribute("competition_module", info.sender)
            .add_attribute("id", id.to_string()))
    }

    pub fn execute_activate(
        &self,
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response, CompetitionError> {
        // Load competition ID associated with the escrow
        let id = self
            .escrows_to_competitions
            .may_load(deps.storage, info.sender.clone())?
            .ok_or(CompetitionError::UnknownEscrow {
                addr: info.sender.to_string(),
            })?;

        // Load competition using the ID
        let mut competition = self
            .competitions
            .may_load(deps.storage, id)?
            .ok_or(CompetitionError::UnknownCompetitionId { id })?;

        // Update competition status
        competition.status = CompetitionStatus::Active;
        self.competitions.save(deps.storage, id, &competition)?;

        Ok(Response::new()
            .add_attribute("id", id.to_string())
            .add_attribute("action", "activate")
            .add_attribute("escrow", info.sender))
    }

    pub fn execute_propose_result(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        propose_message: ProposeMessage,
    ) -> Result<Response, CompetitionError> {
        let id = propose_message.id;

        // Load competition
        let competition = self
            .competitions
            .may_load(deps.storage, id.u128())?
            .ok_or(CompetitionError::UnknownCompetitionId { id: id.u128() })?;

        // Query active proposal modules
        let proposal_modules: Vec<ProposalModule> = deps.querier.query_wasm_smart(
            competition.dao.clone(),
            &dao_interface::msg::QueryMsg::ActiveProposalModules {
                start_after: None,
                limit: None,
            },
        )?;

        // Find a valid proposal module
        let mut proposal_module_addr = None;
        let mut proposer = None;
        for proposal_module in proposal_modules {
            let contract_info = cw2::query_contract_info(&deps.querier, &proposal_module.address);
            if contract_info.is_err()
                || !contract_info
                    .unwrap()
                    .contract
                    .contains("dao-proposal-single")
            {
                continue;
            }

            let creation_policy = deps.querier.query_wasm_smart::<ProposalCreationPolicy>(
                proposal_module.address.clone(),
                &dao_proposal_single::msg::QueryMsg::ProposalCreationPolicy {},
            );
            if creation_policy.is_err()
                || !creation_policy
                    .as_ref()
                    .unwrap()
                    .is_permitted(&env.contract.address)
            {
                continue;
            }

            proposal_module_addr = Some(proposal_module.address);
            proposer = match creation_policy.unwrap() {
                ProposalCreationPolicy::Anyone {} => None,
                ProposalCreationPolicy::Module { addr: _ } => Some(info.sender.to_string()),
            };
            break; // Found a valid proposal module, break out of the loop
        }

        // Ensure a valid proposal module was found
        let proposal_module_addr =
            proposal_module_addr.ok_or(CompetitionError::StdError(StdError::GenericErr {
                msg: "Could not find an accessible dao-proposal-single module".to_owned(),
            }))?;

        // Construct proposal message
        let propose_message = ProposeMessages::Propose(SingleChoiceProposeMsg {
            title: propose_message.title,
            description: propose_message.description,
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_json_binary(&ExecuteBase::<Empty, Empty>::ProcessCompetition {
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
                contract_addr: proposal_module_addr.to_string(),
                msg: to_json_binary(&propose_message)?,
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
        info: MessageInfo,
        propose_message: ProposeMessage,
    ) -> Result<Response, CompetitionError> {
        // Ensure DAO has an owner
        let dao = get_ownership(deps.storage)?;
        let dao_owner = dao.owner.ok_or(CompetitionError::OwnershipError(
            cw_ownable::OwnershipError::NoOwner,
        ))?;

        let id = propose_message.id;

        // Update competition status
        self.competitions.update(deps.storage, id.u128(), |x| {
            let mut competition =
                x.ok_or(CompetitionError::UnknownCompetitionId { id: id.u128() })?;

            // Validate competition status
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

            // Check user membership in the competition DAO
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
        })?;

        // Construct message for the DAO owner
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: dao_owner.to_string(),
            msg: to_json_binary(&arena_core_interface::msg::ExecuteMsg::Propose {
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
        deps: &mut DepsMut,
        env: &Env,
        competition_dao: ModuleInstantiateInfo,
        escrow: Option<ModuleInstantiateInfo>,
        name: String,
        description: String,
        expiration: cw_utils::Expiration,
        rules: Vec<String>,
        rulesets: Vec<Uint128>,
        extension: CompetitionInstantiateExt,
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
            })?
            .checked_add(Uint128::one())?;
        let admin_dao = self.get_dao(deps.as_ref())?;
        let mut competition = Competition {
            admin_dao: admin_dao.clone(),
            start_height: env.block.height,
            id,
            dao: env.contract.address.clone(),
            escrow: None,
            name,
            description,
            expiration,
            rules,
            rulesets,
            status: CompetitionStatus::Pending,
            extension: extension.into(),
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
        } else {
            competition.status = CompetitionStatus::Active;
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
        mut distribution: Vec<cw_balance::MemberShare<String>>,
    ) -> Result<Response, CompetitionError> {
        // Load competition
        let mut competition = self
            .competitions
            .may_load(deps.storage, id.u128())?
            .ok_or(CompetitionError::UnknownCompetitionId { id: id.u128() })?;

        // Validate competition status and sender's authorization
        match competition.status {
            CompetitionStatus::Active => {
                if competition.dao != info.sender && competition.admin_dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
            }
            CompetitionStatus::Jailed => {
                if competition.admin_dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
            }
            _ => {
                return Err(CompetitionError::InvalidCompetitionStatus {
                    current_status: competition.status.clone(),
                })
            }
        }

        // Validate and convert distribution members
        let result = distribution
            .iter()
            .map(|x| x.to_validated(deps.as_ref()))
            .collect::<StdResult<Vec<MemberShare<Addr>>>>()?;

        competition.result = Some(result);
        self.competitions
            .save(deps.storage, id.u128(), &competition)?;

        // Prepare hooks
        let hooks: Vec<(Addr, HookDirection)> = self
            .competition_hooks
            .prefix(id.u128())
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<_>>()?;
        let msg_binary = to_json_binary(&ExecuteBase::<Empty, Empty>::ExecuteCompetitionHook {
            id,
            distribution: distribution.clone(),
        })?;
        let mut msgs: Vec<SubMsg> = hooks
            .iter()
            .filter(|x| x.1 == HookDirection::Outgoing)
            .map(|x| {
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: x.0.to_string(),
                    msg: msg_binary.clone(),
                    funds: vec![],
                }))
            })
            .collect();

        // If there's an escrow, handle distribution and tax
        if let Some(escrow) = competition.escrow {
            if !distribution.is_empty() {
                let arena_core = cw_ownable::get_ownership(deps.storage)?.owner.ok_or(
                    CompetitionError::OwnershipError(cw_ownable::OwnershipError::NoOwner),
                )?;
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
                    let total_shares = distribution
                        .iter()
                        .try_fold(Uint128::zero(), |acc, x| acc.checked_add(x.shares))?;

                    let dao_shares = tax
                        .checked_mul(Decimal::from_atomics(
                            total_shares.checked_mul(precision_multiplier)?,
                            0u32,
                        )?)?
                        .checked_div(Decimal::one().checked_sub(tax)?)?
                        .checked_div(Decimal::from_atomics(
                            Uint128::new(10u128).checked_pow(tax.decimal_places())?,
                            0u32,
                        )?)?
                        .atomics();

                    for member in &mut distribution {
                        member.shares = member.shares.checked_mul(precision_multiplier)?;
                    }

                    distribution.push(MemberShare {
                        addr: competition.admin_dao.to_string(),
                        shares: dao_shares,
                    });
                }
            }

            let sub_msg = SubMsg::reply_on_success(
                CompetitionEscrowDistributeMsg {
                    distribution,
                    remainder_addr: competition.admin_dao.to_string(),
                }
                .into_cosmos_msg(escrow)?,
                PROCESS_REPLY_ID,
            );

            msgs.push(sub_msg);
        }

        Ok(Response::new()
            .add_attribute("action", "process_competition")
            .add_submessages(msgs))
    }

    pub fn query(
        &self,
        deps: Deps,
        env: Env,
        msg: QueryBase<QueryExt, CompetitionExt>,
    ) -> StdResult<Binary> {
        match msg {
            QueryBase::Config {} => to_json_binary(&self.config.load(deps.storage)?),
            QueryBase::Competition { id } => to_json_binary(
                &self
                    .competitions
                    .load(deps.storage, id.u128())?
                    .to_response(&env.block),
            ),
            QueryBase::Competitions {
                start_after,
                limit,
                status,
            } => to_json_binary(&self.query_competitions(deps, env, start_after, limit, status)?),
            QueryBase::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
            QueryBase::CompetitionCount {} => {
                to_json_binary(&self.competition_count.load(deps.storage)?)
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
                |_x, y| Ok(y.to_response(&env.block)),
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
                .map(|x| x.map(|y| y.1.to_response(&env.block)))
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
        self.escrows_to_competitions
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
