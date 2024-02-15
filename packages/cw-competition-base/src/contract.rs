use std::marker::PhantomData;

use arena_core_interface::msg::{CompetitionModuleResponse, ProposeMessage};
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty,
    Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw_balance::MemberPercentage;
use cw_competition::{
    escrow::{CompetitionEscrowDistributeMsg, TaxInformation},
    msg::{
        CompetitionsFilter, ExecuteBase, HookDirection, InstantiateBase, IntoCompetitionExt,
        ModuleInfo, QueryBase,
    },
    state::{Competition, CompetitionResponse, CompetitionStatus, Config, Evidence},
};
use cw_ownable::{get_ownership, initialize_owner};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use dao_interface::state::ModuleInstantiateInfo;
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CompetitionError;

pub const PROCESS_REPLY_ID: u64 = 1;

pub struct CompetitionIndexes<'a, CompetitionExt> {
    pub status: MultiIndex<'a, String, Competition<CompetitionExt>, u128>,
    pub category: MultiIndex<'a, u128, Competition<CompetitionExt>, u128>,
}

impl<'a, CompetitionExt: Serialize + Clone + DeserializeOwned>
    IndexList<Competition<CompetitionExt>> for CompetitionIndexes<'a, CompetitionExt>
{
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<Competition<CompetitionExt>>> + '_> {
        let v: Vec<&dyn Index<Competition<CompetitionExt>>> = vec![&self.status, &self.category];
        Box::new(v.into_iter())
    }
}

pub struct CompetitionModuleContract<
    InstantiateExt,
    ExecuteExt,
    QueryExt,
    CompetitionExt: Serialize + Clone + DeserializeOwned,
    CompetitionInstantiateExt: IntoCompetitionExt<CompetitionExt>,
> {
    pub config: Item<'static, Config<InstantiateExt>>,
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
        InstantiateExt: Serialize + DeserializeOwned + Clone,
        ExecuteExt,
        QueryExt: JsonSchema,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
        CompetitionInstantiateExt: IntoCompetitionExt<CompetitionExt>,
    >
    CompetitionModuleContract<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt,
        CompetitionInstantiateExt,
    >
{
    #[allow(clippy::too_many_arguments)]
    const fn new(
        config_key: &'static str,
        competition_count_key: &'static str,
        competitions_key: &'static str,
        competitions_status_key: &'static str,
        competitions_category_key: &'static str,
        escrows_to_competitions_key: &'static str,
        temp_competition_key: &'static str,
        competition_hooks_key: &'static str,
    ) -> Self {
        Self {
            config: Item::new(config_key),
            competition_count: Item::new(competition_count_key),
            competitions: Self::competitions(
                competitions_key,
                competitions_status_key,
                competitions_category_key,
            ),
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
        competitions_category_key: &'static str,
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
            category: MultiIndex::new(
                |_x, d: &Competition<CompetitionExt>| d.category_id.u128(),
                competitions_key,
                competitions_category_key,
            ),
        };
        IndexedMap::new(competitions_key, indexes)
    }
}

impl<
        InstantiateExt: Serialize + DeserializeOwned + Clone,
        ExecuteExt,
        QueryExt: JsonSchema,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
        CompetitionInstantiateExt: IntoCompetitionExt<CompetitionExt>,
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
            "competitions__category",
            "escrows",
            "temp_competition",
            "competition_hooks",
        )
    }
}

impl<
        InstantiateExt: Serialize + DeserializeOwned + Clone,
        ExecuteExt,
        QueryExt: JsonSchema,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
        CompetitionInstantiateExt: IntoCompetitionExt<CompetitionExt>,
    >
    CompetitionModuleContract<
        InstantiateExt,
        ExecuteExt,
        QueryExt,
        CompetitionExt,
        CompetitionInstantiateExt,
    >
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
                extension: msg.extension,
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
                category_id,
                host: competition_dao,
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
                category_id,
                competition_dao,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                instantiate_extension,
            ),
            ExecuteBase::ProcessCompetition {
                id,
                distribution,
                cw20_msg,
                cw721_msg,
            } => {
                self.execute_process_competition(deps, info, id, distribution, cw20_msg, cw721_msg)
            }
            ExecuteBase::UpdateOwnership(action) => {
                let ownership =
                    cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
                Ok(Response::new().add_attributes(ownership.into_attributes()))
            }
            ExecuteBase::Activate {} => self.execute_activate(deps, info),
            ExecuteBase::SubmitEvidence { id, evidence } => {
                self.execute_submit_evidence(deps, env, info, id, evidence)
            }
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

    pub fn execute_submit_evidence(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        id: Uint128,
        evidence: Vec<String>,
    ) -> Result<Response, CompetitionError> {
        self.competitions.update(
            deps.storage,
            id.u128(),
            |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        if competition.status != CompetitionStatus::Jailed {
                            return Err(CompetitionError::InvalidCompetitionStatus {
                                current_status: competition.status,
                            });
                        }

                        competition
                            .evidence
                            .extend(evidence.iter().map(|x| Evidence {
                                submit_user: info.sender.clone(),
                                content: x.to_string(),
                                submit_time: env.block.time,
                            }));
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id: id.u128() }),
                }
            },
        )?;

        Ok(Response::new()
            .add_attribute("action", "submit_evidence")
            .add_attribute("sender", info.sender.to_string()))
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

    pub fn execute_jail_competition(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        propose_message: ProposeMessage,
    ) -> Result<Response, CompetitionError> {
        // Ensure Module has an owner
        let ownership = get_ownership(deps.storage)?;
        let arena_core = ownership.owner.ok_or(CompetitionError::OwnershipError(
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
                        competition.host.clone(),
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
            contract_addr: arena_core.to_string(),
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
        category_id: Uint128,
        host: ModuleInfo,
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

        // Ensure Module has an owner
        let ownership = get_ownership(deps.storage)?;
        let arena_core = ownership.owner.ok_or(CompetitionError::OwnershipError(
            cw_ownable::OwnershipError::NoOwner,
        ))?;

        // Setup
        let id = self
            .competition_count
            .update(deps.storage, |x| -> StdResult<_> {
                Ok(x.checked_add(Uint128::one())?)
            })?;
        let admin_dao = self.get_dao(deps.as_ref())?;
        let mut msgs = vec![];
        let mut initial_status = CompetitionStatus::Pending;

        // Declare instantiate2 vars
        let salt = env.block.height.to_ne_bytes();
        let canonical_creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
        let host_addr = match host {
            ModuleInfo::New { info } => {
                let code_info = deps.querier.query_wasm_code_info(info.code_id)?;
                let canonical_addr =
                    instantiate2_address(&code_info.checksum, &canonical_creator, &salt)?;

                msgs.push(WasmMsg::Instantiate2 {
                    admin: info.admin.map(|admin| match admin {
                        dao_interface::state::Admin::Address { addr } => addr,
                        dao_interface::state::Admin::CoreModule {} => admin_dao.to_string(),
                    }),
                    code_id: info.code_id,
                    label: info.label,
                    msg: info.msg,
                    funds: vec![],
                    salt: salt.into(),
                });

                deps.api.addr_humanize(&canonical_addr)
            }
            ModuleInfo::Existing { addr } => deps.api.addr_validate(&addr),
        }?;
        let escrow_addr = match escrow {
            Some(info) => {
                let code_info = deps.querier.query_wasm_code_info(info.code_id)?;
                let canonical_addr =
                    instantiate2_address(&code_info.checksum, &canonical_creator, &salt)?;

                msgs.push(WasmMsg::Instantiate2 {
                    admin: Some(host_addr.to_string()),
                    code_id: info.code_id,
                    label: info.label,
                    msg: info.msg,
                    funds: vec![],
                    salt: salt.into(),
                });

                let addr = deps.api.addr_humanize(&canonical_addr)?;

                self.escrows_to_competitions
                    .save(deps.storage, addr.clone(), &id.u128())?;

                Some(addr)
            }
            None => {
                initial_status = CompetitionStatus::Active;
                None
            }
        };

        // Validate that category and rulesets are valid
        let result: bool = deps.querier.query_wasm_smart(
            arena_core,
            &arena_core_interface::msg::QueryMsg::QueryExtension {
                msg: arena_core_interface::msg::QueryExt::IsValidCategoryAndRulesets {
                    category_id,
                    rulesets: rulesets.clone(),
                },
            },
        )?;
        if !result {
            return Err(CompetitionError::InvalidCategoryAndRulesets {
                category_id,
                rulesets,
            });
        }

        // Create competition
        let competition = Competition {
            id,
            category_id,
            admin_dao: admin_dao.clone(),
            host: host_addr,
            start_height: env.block.height,
            escrow: escrow_addr,
            name,
            description,
            expiration,
            rules,
            rulesets,
            status: initial_status,
            extension: extension.into_competition_ext(deps.as_ref())?,
            result: None,
            evidence: vec![],
        };

        self.competitions
            .save(deps.storage, id.u128(), &competition)?;

        Ok(Response::new()
            .add_attribute("action", "create_competition")
            .add_attribute("id", id)
            .add_attribute(
                "escrow_addr",
                competition
                    .escrow
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
            )
            .add_attribute("host", competition.host)
            .add_messages(msgs))
    }

    pub fn execute_process_competition(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: Uint128,
        distribution: Vec<cw_balance::MemberPercentage<String>>,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> Result<Response, CompetitionError> {
        // Load competition
        let mut competition = self
            .competitions
            .may_load(deps.storage, id.u128())?
            .ok_or(CompetitionError::UnknownCompetitionId { id: id.u128() })?;

        // Validate competition status and sender's authorization
        match competition.status {
            CompetitionStatus::Active => {
                if competition.host != info.sender && competition.admin_dao != info.sender {
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

        // Validate the distribution
        let result = distribution
            .iter()
            .map(|x| x.into_checked(deps.as_ref()))
            .collect::<StdResult<Vec<MemberPercentage<Addr>>>>()?;

        let sum = distribution
            .iter()
            .try_fold(Decimal::zero(), |acc, x| acc.checked_add(x.percentage))?;

        if sum != Decimal::one() {
            return Err(CompetitionError::InvalidDistribution {});
        }

        // Set the result
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
            let tax_info = if !distribution.is_empty() {
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
                    Some(TaxInformation {
                        tax,
                        receiver: competition.admin_dao.to_string(),
                        cw20_msg,
                        cw721_msg,
                    })
                } else {
                    None
                }
            } else {
                None
            };

            let sub_msg = SubMsg::reply_on_success(
                CompetitionEscrowDistributeMsg {
                    distribution,
                    tax_info,
                    remainder_addr: competition.admin_dao.to_string(),
                }
                .into_cosmos_msg(escrow.clone())?,
                PROCESS_REPLY_ID,
            );

            self.temp_competition.save(deps.storage, &id.u128())?;

            // We don't expect another activation message from the escrow
            self.escrows_to_competitions.remove(deps.storage, escrow);

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
        msg: QueryBase<InstantiateExt, QueryExt, CompetitionExt>,
    ) -> StdResult<Binary> {
        match msg {
            QueryBase::Config {} => to_json_binary(&self.config.load(deps.storage)?),
            QueryBase::Competition { id } => to_json_binary(
                &self
                    .competitions
                    .load(deps.storage, id.u128())?
                    .into_response(&env.block),
            ),
            QueryBase::DAO {} => to_json_binary(
                &self
                    .get_dao(deps)
                    .map_err(|x| StdError::GenericErr { msg: x.to_string() })?,
            ),
            QueryBase::Competitions {
                start_after,
                limit,
                filter,
            } => to_json_binary(&self.query_competitions(deps, env, start_after, limit, filter)?),
            QueryBase::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
            QueryBase::CompetitionCount {} => {
                to_json_binary(&self.competition_count.load(deps.storage)?)
            }
            QueryBase::QueryExtension { .. } => Ok(Binary::default()),
            QueryBase::_Phantom(_) => Ok(Binary::default()),
        }
    }

    pub fn get_dao(&self, deps: Deps) -> Result<Addr, cw_ownable::OwnershipError> {
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
        filter: Option<CompetitionsFilter>,
    ) -> StdResult<Vec<CompetitionResponse<CompetitionExt>>> {
        let start_after_bound = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(10).max(30);

        match filter {
            None => cw_paginate::paginate_indexed_map(
                &self.competitions,
                deps.storage,
                start_after_bound,
                Some(limit),
                |_x, y| Ok(y.into_response(&env.block)),
            ),
            Some(filter) => match filter {
                CompetitionsFilter::CompetitionStatus { status } => self
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
                    .map(|x| x.map(|y| y.1.into_response(&env.block)))
                    .take(limit as usize)
                    .collect::<StdResult<Vec<_>>>(),
                CompetitionsFilter::Category { id } => self
                    .competitions
                    .idx
                    .category
                    .prefix(id.u128())
                    .range(
                        deps.storage,
                        start_after_bound,
                        None,
                        cosmwasm_std::Order::Ascending,
                    )
                    .map(|x| x.map(|y| y.1.into_response(&env.block)))
                    .take(limit as usize)
                    .collect::<StdResult<Vec<_>>>(),
            },
        }
    }

    pub fn reply(
        &self,
        deps: DepsMut,
        _env: Env,
        msg: Reply,
    ) -> Result<Response, CompetitionError> {
        match msg.id {
            PROCESS_REPLY_ID => self.reply_process(deps, msg),
            _ => Err(CompetitionError::UnknownReplyId { id: msg.id }),
        }
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
