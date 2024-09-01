use std::marker::PhantomData;

use arena_interface::{
    competition::{
        msg::{
            CompetitionsFilter, EscrowInstantiateInfo, ExecuteBase, HookDirection, InstantiateBase,
            MemberStatUpdate, QueryBase, StatMsg, ToCompetitionExt,
        },
        state::{
            Competition, CompetitionResponse, CompetitionStatus, Config, Evidence, StatType,
            StatValue,
        },
    },
    core::{CompetitionModuleResponse, ProposeMessage, TaxConfigurationResponse},
    fees::FeeInformation,
    ratings::MemberResult,
};
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    ensure, instantiate2_address, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty,
    Env, MessageInfo, Order, Reply, Response, StdError, StdResult, Storage, SubMsg, Uint128,
    WasmMsg,
};
use cw_balance::Distribution;
use cw_ownable::{assert_owner, get_ownership, initialize_owner};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CompetitionError;

pub const PROCESS_REPLY_ID: u64 = 1;
pub const UPDATE_RATING_FAILED_REPLY_ID: u64 = 2;

pub struct CompetitionIndexes<'a, CompetitionExt> {
    pub status: MultiIndex<'a, String, Competition<CompetitionExt>, u128>,
    pub category: MultiIndex<'a, u128, Competition<CompetitionExt>, u128>,
    pub host: MultiIndex<'a, String, Competition<CompetitionExt>, u128>,
}

impl<'a, CompetitionExt: Serialize + Clone + DeserializeOwned>
    IndexList<Competition<CompetitionExt>> for CompetitionIndexes<'a, CompetitionExt>
{
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<Competition<CompetitionExt>>> + '_> {
        let v: Vec<&dyn Index<Competition<CompetitionExt>>> =
            vec![&self.status, &self.category, &self.host];
        Box::new(v.into_iter())
    }
}

pub struct CompetitionModuleContract<
    'a,
    InstantiateExt,
    ExecuteExt,
    QueryExt,
    CompetitionExt: Serialize + Clone + DeserializeOwned,
    CompetitionInstantiateExt: ToCompetitionExt<CompetitionExt>,
> {
    pub config: Item<'static, Config<InstantiateExt>>,
    pub competition_count: Item<'static, Uint128>,
    pub competitions: IndexedMap<
        'static,
        u128,
        Competition<CompetitionExt>,
        CompetitionIndexes<'static, CompetitionExt>,
    >,
    pub competition_evidence: Map<'static, (u128, u128), Evidence>,
    pub competition_evidence_count: Map<'static, u128, Uint128>,
    pub competition_result: Map<'static, u128, Option<Distribution<Addr>>>,
    pub competition_rules: Map<'static, u128, Vec<String>>,
    pub escrows_to_competitions: Map<'static, &'a Addr, u128>,
    pub temp_competition: Item<'static, u128>,
    pub competition_hooks: Map<'static, (u128, &'a Addr), HookDirection>,
    pub stats: Map<'static, (u128, &'a Addr, &'a str), StatValue>,
    pub stat_types: Map<'a, (u128, &'a str), StatType>,

    instantiate_type: PhantomData<InstantiateExt>,
    execute_type: PhantomData<ExecuteExt>,
    query_type: PhantomData<QueryExt>,
    competition_instantiate_type: PhantomData<CompetitionInstantiateExt>,
}

impl<
        'a,
        InstantiateExt: Serialize + DeserializeOwned + Clone,
        ExecuteExt,
        QueryExt: JsonSchema,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
        CompetitionInstantiateExt: ToCompetitionExt<CompetitionExt>,
    >
    CompetitionModuleContract<
        'a,
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
        competitions_host_key: &'static str,
        escrows_to_competitions_key: &'static str,
        temp_competition_key: &'static str,
        competition_hooks_key: &'static str,
        competition_evidence_key: &'static str,
        competition_evidence_count_key: &'static str,
        competition_result_key: &'static str,
        competition_rules_key: &'static str,
        stats_key: &'static str,
        stat_types_key: &'static str,
    ) -> Self {
        Self {
            config: Item::new(config_key),
            competition_count: Item::new(competition_count_key),
            competitions: Self::competitions(
                competitions_key,
                competitions_status_key,
                competitions_category_key,
                competitions_host_key,
            ),
            escrows_to_competitions: Map::new(escrows_to_competitions_key),
            temp_competition: Item::new(temp_competition_key),
            competition_hooks: Map::new(competition_hooks_key),
            competition_evidence: Map::new(competition_evidence_key),
            competition_evidence_count: Map::new(competition_evidence_count_key),
            competition_result: Map::new(competition_result_key),
            competition_rules: Map::new(competition_rules_key),
            stats: Map::new(stats_key),
            stat_types: Map::new(stat_types_key),
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
        competitions_host_key: &'static str,
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
                |_x, d: &Competition<CompetitionExt>| {
                    d.category_id.unwrap_or(Uint128::zero()).u128()
                },
                competitions_key,
                competitions_category_key,
            ),
            host: MultiIndex::new(
                |_x, d: &Competition<CompetitionExt>| d.host.to_string(),
                competitions_key,
                competitions_host_key,
            ),
        };
        IndexedMap::new(competitions_key, indexes)
    }
}

impl<
        'a,
        InstantiateExt: Serialize + DeserializeOwned + Clone,
        ExecuteExt,
        QueryExt: JsonSchema,
        CompetitionExt: Serialize + Clone + DeserializeOwned,
        CompetitionInstantiateExt: ToCompetitionExt<CompetitionExt>,
    > Default
    for CompetitionModuleContract<
        'a,
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
            "competitions__host",
            "escrows_to_competitions",
            "temp_competition",
            "competition_hooks",
            "competition_evidence",
            "competition_evidence_count",
            "competition_result",
            "competition_rules",
            "stats",
            "stat_types",
        )
    }
}

impl<
        'a,
        InstantiateExt: Serialize + DeserializeOwned + Clone + std::fmt::Debug,
        ExecuteExt,
        QueryExt: JsonSchema,
        CompetitionExt: Serialize + Clone + DeserializeOwned + std::fmt::Debug,
        CompetitionInstantiateExt: ToCompetitionExt<CompetitionExt>,
    >
    CompetitionModuleContract<
        'a,
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
        _env: Env,
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
            ExecuteBase::JailCompetition {
                competition_id,
                title,
                description,
                distribution,
                additional_layered_fees,
            } => self.execute_jail_competition(
                deps,
                env,
                info,
                competition_id,
                title,
                description,
                distribution,
                additional_layered_fees,
            ),
            ExecuteBase::CreateCompetition {
                host,
                category_id,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                instantiate_extension,
            } => self.execute_create_competition(
                &mut deps,
                &env,
                &info,
                host,
                category_id,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                &instantiate_extension,
            ),
            ExecuteBase::ProcessCompetition {
                competition_id,
                distribution,
            } => self.execute_process_competition(deps, info, competition_id, distribution, None),
            ExecuteBase::UpdateOwnership(action) => {
                let ownership =
                    cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
                Ok(Response::new().add_attributes(ownership.into_attributes()))
            }
            ExecuteBase::ActivateCompetition {} => {
                self.execute_activate_from_escrow(deps, env, info)
            }
            ExecuteBase::SubmitEvidence {
                competition_id: id,
                evidence,
            } => self.execute_submit_evidence(deps, env, info, id, evidence),
            ExecuteBase::AddCompetitionHook { competition_id } => {
                self.execute_add_competition_hook(deps, info, competition_id)
            }
            ExecuteBase::RemoveCompetitionHook { competition_id } => {
                self.execute_remove_competition_hook(deps, info, competition_id)
            }
            ExecuteBase::MigrateEscrows {
                start_after,
                limit,
                filter,
                escrow_code_id,
                escrow_migrate_msg,
            } => self.execute_migrate_escrows(
                deps,
                env,
                info,
                start_after,
                limit,
                filter,
                escrow_code_id,
                escrow_migrate_msg,
            ),
            ExecuteBase::UpdateStatTypes {
                competition_id,
                to_add,
                to_remove,
            } => self.execute_update_stat_types(deps, env, info, competition_id, to_add, to_remove),
            ExecuteBase::UpdateStats {
                competition_id,
                updates,
            } => self.execute_update_stats(deps, env, info, competition_id, updates),
            ExecuteBase::ExecuteCompetitionHook {
                competition_id: _,
                distribution: _,
            }
            | ExecuteBase::Extension { .. } => Ok(Response::default()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_migrate_escrows(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        start_after: Option<Uint128>,
        limit: Option<u32>,
        filter: Option<CompetitionsFilter>,
        escrow_code_id: u64,
        escrow_migrate_msg: arena_interface::escrow::MigrateMsg,
    ) -> Result<Response, CompetitionError> {
        // Ensure only the contract owner can call this function
        assert_owner(deps.storage, &info.sender)?;

        let competitions =
            self.query_competitions(deps.as_ref(), env, start_after, limit, filter)?;

        let mut messages: Vec<SubMsg> = vec![];

        for competition in competitions.iter() {
            if let Some(escrow) = &competition.escrow {
                let msg = WasmMsg::Migrate {
                    contract_addr: escrow.to_string(),
                    new_code_id: escrow_code_id,
                    msg: to_json_binary(&escrow_migrate_msg)?,
                };

                messages.push(SubMsg::new(msg));
            }
        }

        let length = messages.len();
        Ok(Response::new()
            .add_submessages(messages)
            .add_attribute("action", "migrate_escrows")
            .add_attribute("migrated_count", length.to_string()))
    }

    pub fn execute_submit_evidence(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        competition_id: Uint128,
        evidence: Vec<String>,
    ) -> Result<Response, CompetitionError> {
        let competition = self
            .competitions
            .load(deps.storage, competition_id.u128())?;

        // Validate that competition is jailed
        if competition.status != CompetitionStatus::Jailed {
            return Err(CompetitionError::InvalidCompetitionStatus {
                current_status: competition.status,
            });
        }

        let mut evidence_id = self
            .competition_evidence_count
            .may_load(deps.storage, competition_id.u128())?
            .unwrap_or_default();

        for item in evidence {
            self.competition_evidence.save(
                deps.storage,
                (competition_id.u128(), evidence_id.u128()),
                &Evidence {
                    id: evidence_id,
                    submit_user: info.sender.clone(),
                    content: item.to_string(),
                    submit_time: env.block.time,
                },
            )?;

            evidence_id = evidence_id.checked_add(Uint128::one())?;
        }

        self.competition_evidence_count
            .save(deps.storage, competition_id.u128(), &evidence_id)?;

        Ok(Response::new()
            .add_attribute("action", "submit_evidence")
            .add_attribute("sender", info.sender.to_string())
            .add_attribute("evidence_count", evidence_id))
    }

    pub fn validate_execute_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        competition_id: Uint128,
    ) -> Result<(), CompetitionError> {
        // Validate hook
        if HookDirection::Incoming
            != self
                .competition_hooks
                .load(deps.storage, (competition_id.u128(), &info.sender))?
        {
            return Err(CompetitionError::Unauthorized {});
        }

        Ok(())
    }

    pub fn execute_add_competition_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        competition_id: Uint128,
    ) -> Result<Response, CompetitionError> {
        // Load competition using the ID
        if !self.competitions.has(deps.storage, competition_id.u128()) {
            return Err(CompetitionError::UnknownCompetitionId { id: competition_id });
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
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: arena_interface::core::QueryExt::CompetitionModule {
                    query: arena_interface::core::CompetitionModuleQuery::Addr(
                        info.sender.to_string(),
                    ),
                },
            },
        )?;

        if !competition_module.is_enabled {
            return Err(CompetitionError::StdError(StdError::generic_err(
                "Competition module is not enabled",
            )));
        }

        // Add competition hook
        self.competition_hooks.save(
            deps.storage,
            (competition_id.u128(), &info.sender),
            &HookDirection::Outgoing,
        )?;

        Ok(Response::new()
            .add_attribute("action", "add_competition_hook")
            .add_attribute("competition_module", info.sender)
            .add_attribute("id", competition_id.to_string()))
    }

    pub fn execute_remove_competition_hook(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        competition_id: Uint128,
    ) -> Result<Response, CompetitionError> {
        // Load competition using the ID
        if !self.competitions.has(deps.storage, competition_id.u128()) {
            return Err(CompetitionError::UnknownCompetitionId { id: competition_id });
        };

        // Remove competition hook
        self.competition_hooks
            .remove(deps.storage, (competition_id.u128(), &info.sender));

        Ok(Response::new()
            .add_attribute("action", "add_competition_hook")
            .add_attribute("competition_module", info.sender)
            .add_attribute("id", competition_id.to_string()))
    }

    pub fn execute_activate_from_escrow(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
    ) -> Result<Response, CompetitionError> {
        // Load competition ID associated with the escrow
        let id = self
            .escrows_to_competitions
            .may_load(deps.storage, &info.sender)?
            .ok_or(CompetitionError::UnknownEscrow {
                addr: info.sender.to_string(),
            })?;

        // Load competition using the ID
        let competition = self.competitions.may_load(deps.storage, id)?.ok_or(
            CompetitionError::UnknownCompetitionId {
                id: Uint128::new(id),
            },
        )?;

        // Update competition status
        let new_competition = Competition {
            status: CompetitionStatus::Active {
                height: env.block.height,
            },
            ..competition.clone()
        };

        self.competitions
            .replace(deps.storage, id, Some(&new_competition), Some(&competition))?;

        // Do not expect another activation message
        self.escrows_to_competitions
            .remove(deps.storage, &info.sender);

        Ok(Response::new()
            .add_attribute("id", id.to_string())
            .add_attribute("action", "activate"))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_jail_competition(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        competition_id: Uint128,
        title: String,
        description: String,
        distribution: Option<Distribution<String>>,
        additional_layered_fees: Option<FeeInformation<String>>,
    ) -> Result<Response, CompetitionError> {
        // Ensure Module has an owner
        let ownership = get_ownership(deps.storage)?;
        let arena_core = ownership.owner.ok_or(CompetitionError::OwnershipError(
            cw_ownable::OwnershipError::NoOwner,
        ))?;

        // Update competition status
        self.competitions
            .update(deps.storage, competition_id.u128(), |x| {
                let mut competition =
                    x.ok_or(CompetitionError::UnknownCompetitionId { id: competition_id })?;

                // Validate competition status
                if competition.status != CompetitionStatus::Jailed {
                    if !matches!(competition.status, CompetitionStatus::Active { .. }) {
                        return Err(CompetitionError::InvalidCompetitionStatus {
                            current_status: competition.status,
                        });
                    }
                    if !competition.expiration.is_expired(&env.block) {
                        return Err(CompetitionError::CompetitionNotExpired {});
                    }
                }

                competition.status = CompetitionStatus::Jailed;
                Ok(competition)
            })?;

        // Create the proposal
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena_core.to_string(),
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Propose {
                msg: ProposeMessage {
                    competition_id,
                    title,
                    description,
                    distribution,
                    additional_layered_fees,
                    originator: info.sender.to_string(),
                },
            })?,
            funds: info.funds,
        });

        Ok(Response::new()
            .add_attribute("action", "jail_wager")
            .add_attribute("competition_id", competition_id)
            .add_attribute("originator", info.sender)
            .add_message(msg))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_create_competition(
        &self,
        deps: &mut DepsMut,
        env: &Env,
        info: &MessageInfo,
        host: Option<String>,
        category_id: Option<Uint128>,
        escrow: Option<EscrowInstantiateInfo>,
        name: String,
        description: String,
        expiration: cw_utils::Expiration,
        rules: Option<Vec<String>>,
        rulesets: Option<Vec<Uint128>>,
        banner: Option<String>,
        extension: &CompetitionInstantiateExt,
    ) -> Result<Response, CompetitionError> {
        // Validate expiration
        if expiration.is_expired(&env.block) {
            return Err(CompetitionError::StdError(StdError::generic_err(
                "Cannot create an expired competition",
            )));
        }

        // Ensure Module has an owner
        let ownership = get_ownership(deps.storage)?;
        let arena_core = ownership.owner.ok_or(CompetitionError::OwnershipError(
            cw_ownable::OwnershipError::NoOwner,
        ))?;

        // Determine host
        let host = if let Some(host) = host {
            let is_enrollment_module: bool = deps.querier.query_wasm_smart(
                arena_core.to_string(),
                &arena_interface::core::QueryMsg::QueryExtension {
                    msg: arena_interface::core::QueryExt::IsValidEnrollmentModule {
                        addr: info.sender.to_string(),
                    },
                },
            )?;

            ensure!(
                is_enrollment_module,
                CompetitionError::StdError(StdError::generic_err(
                    "Only a valid enrollment module can specify a host."
                ))
            );

            deps.api.addr_validate(&host)?
        } else {
            info.sender.clone()
        };

        // Increment competition count
        let competition_id = self
            .competition_count
            .update(deps.storage, |x| -> StdResult<_> {
                Ok(x.checked_add(Uint128::one())?)
            })?;

        let admin_dao = self.query_dao(deps.as_ref())?;
        let mut msgs = vec![];

        // Handle escrow setup
        let (escrow_addr, fees) = if let Some(escrow_info) = escrow {
            let salt = env.block.height.to_ne_bytes();
            let canonical_creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
            let code_info = deps.querier.query_wasm_code_info(escrow_info.code_id)?;
            let canonical_addr =
                instantiate2_address(&code_info.checksum, &canonical_creator, &salt)?;

            msgs.push(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
                admin: Some(env.contract.address.to_string()),
                code_id: escrow_info.code_id,
                label: escrow_info.label,
                msg: escrow_info.msg,
                funds: vec![],
                salt: salt.into(),
            }));

            let escrow_addr = deps.api.addr_humanize(&canonical_addr)?;
            self.escrows_to_competitions.save(
                deps.storage,
                &escrow_addr,
                &competition_id.u128(),
            )?;

            let fees = escrow_info
                .additional_layered_fees
                .map(|fees| {
                    fees.iter()
                        .map(|fee| fee.into_checked(deps.as_ref()))
                        .collect::<StdResult<Vec<_>>>()
                })
                .transpose()?;

            (Some(escrow_addr), fees)
        } else {
            (None, None)
        };

        // Validate category and rulesets
        if let Some(category_id) = category_id {
            if let Some(rulesets) = rulesets.as_ref() {
                if !rulesets.is_empty() {
                    let is_valid: bool = deps.querier.query_wasm_smart(
                        arena_core,
                        &arena_interface::core::QueryMsg::QueryExtension {
                            msg: arena_interface::core::QueryExt::IsValidCategoryAndRulesets {
                                category_id,
                                rulesets: rulesets.clone(),
                            },
                        },
                    )?;
                    if !is_valid {
                        return Err(CompetitionError::InvalidCategoryAndRulesets {
                            category_id,
                            rulesets: rulesets.to_vec(),
                        });
                    }
                }
            }
        }

        // Create competition
        let competition = Competition {
            id: competition_id,
            category_id,
            admin_dao,
            host,
            start_height: env.block.height,
            escrow: escrow_addr,
            name,
            description,
            expiration,
            rulesets,
            status: CompetitionStatus::Pending,
            extension: extension.to_competition_ext(deps.as_ref())?,
            fees,
            banner,
        };

        // Save competition data
        if let Some(rules) = rules {
            self.competition_rules
                .save(deps.storage, competition_id.u128(), &rules)?;
        }
        self.competitions
            .save(deps.storage, competition_id.u128(), &competition)?;

        Ok(Response::new()
            .add_attribute("action", "create_competition")
            .add_attribute("competition_id", competition_id)
            .add_attribute(
                "escrow_addr",
                competition
                    .escrow
                    .map_or_else(|| "None".to_string(), |addr| addr.to_string()),
            )
            .add_attribute("host", competition.host)
            .add_messages(msgs))
    }

    #[allow(clippy::type_complexity)]
    pub fn execute_process_competition(
        &self,
        mut deps: DepsMut,
        info: MessageInfo,
        competition_id: Uint128,
        distribution: Option<Distribution<String>>,
        post_processing: Option<
            fn(
                deps: DepsMut,
                &Competition<CompetitionExt>,
            ) -> Result<Option<SubMsg>, CompetitionError>,
        >,
    ) -> Result<Response, CompetitionError> {
        // Load competition
        let competition = self
            .competitions
            .may_load(deps.storage, competition_id.u128())?
            .ok_or(CompetitionError::UnknownCompetitionId { id: competition_id })?;

        // Validate competition status and sender's authorization
        self.inner_validate_auth(&info.sender, &competition)?;

        // Validate the distribution
        let validated_distribution = distribution
            .as_ref()
            .map(|some| some.into_checked(deps.as_ref()))
            .transpose()?;

        // Process the competition
        let mut response =
            self.inner_process(deps.branch(), &competition, validated_distribution)?;

        // Post-processing
        if let Some(post_processing) = post_processing {
            if let Some(sub_msg) = post_processing(deps.branch(), &competition)? {
                response = response.add_submessage(sub_msg);
            }
        }

        Ok(response)
    }

    // Validate competition status and sender's authorization
    pub fn inner_validate_auth(
        &self,
        sender: &Addr,
        competition: &Competition<CompetitionExt>,
    ) -> Result<(), CompetitionError> {
        match competition.status {
            CompetitionStatus::Active { height: _ } => {
                if competition.host != sender && competition.admin_dao != sender {
                    return Err(CompetitionError::Unauthorized {});
                }
            }
            CompetitionStatus::Jailed => {
                if competition.admin_dao != sender {
                    return Err(CompetitionError::Unauthorized {});
                }
            }
            _ => {
                return Err(CompetitionError::InvalidCompetitionStatus {
                    current_status: competition.status.clone(),
                })
            }
        }

        Ok(())
    }

    // Process a competition
    pub fn inner_process(
        &self,
        deps: DepsMut,
        competition: &Competition<CompetitionExt>,
        distribution: Option<Distribution<Addr>>,
    ) -> Result<Response, CompetitionError> {
        // Set the result
        self.competition_result
            .save(deps.storage, competition.id.u128(), &distribution)?;

        // Get a distribution for messaging
        let distribution_msg = distribution.as_ref().map(|x| x.into_unchecked());

        // Prepare hooks
        let hooks: Vec<(Addr, HookDirection)> = self
            .competition_hooks
            .prefix(competition.id.u128())
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<_>>()?;
        let msg_binary = to_json_binary(&ExecuteBase::<Empty, Empty>::ExecuteCompetitionHook {
            competition_id: competition.id,
            distribution: distribution_msg.clone(),
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

        // If there's an escrow, handle distribution, tax, and fees
        if let Some(escrow) = &competition.escrow {
            // Get Arena Tax config
            let arena_tax_config =
                self.query_arena_tax_config(deps.as_ref(), competition.start_height)?;

            let mut layered_fees = vec![];

            // Apply Arena Tax
            if !arena_tax_config.tax.is_zero() {
                layered_fees.push(FeeInformation {
                    tax: arena_tax_config.tax,
                    receiver: competition.admin_dao.to_string(),
                    cw20_msg: arena_tax_config.cw20_msg.clone(),
                    cw721_msg: arena_tax_config.cw721_msg.clone(),
                });
            }

            // Apply additional layered fees
            if let Some(additional_layered_fees) = &competition.fees {
                layered_fees.extend(additional_layered_fees.iter().map(|x| FeeInformation {
                    tax: x.tax,
                    receiver: x.receiver.to_string(),
                    cw20_msg: x.cw20_msg.clone(),
                    cw721_msg: x.cw721_msg.clone(),
                }));
            }

            let layered_fees = if layered_fees.is_empty() {
                None
            } else {
                Some(layered_fees)
            };

            match competition.status {
                CompetitionStatus::Active { height } => {
                    let sub_msg = SubMsg::reply_on_success(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: escrow.to_string(),
                            msg: to_json_binary(
                                &arena_interface::escrow::ExecuteMsg::Distribute {
                                    distribution: distribution_msg,
                                    layered_fees,
                                    activation_height: Some(height),
                                },
                            )?,
                            funds: vec![],
                        }),
                        PROCESS_REPLY_ID,
                    );

                    self.temp_competition
                        .save(deps.storage, &competition.id.u128())?;

                    msgs.push(sub_msg);

                    Ok(())
                }
                _ => Err(CompetitionError::InvalidCompetitionStatus {
                    current_status: competition.status.clone(),
                }),
            }?;
        }

        // Tax info is displayed in the escrow response
        Ok(Response::new()
            .add_attribute("action", "process_competition")
            .add_attribute(
                "distribution",
                distribution
                    .map(|some| some.to_string())
                    .unwrap_or("None".to_owned()),
            )
            .add_submessages(msgs))
    }

    // This method is meant to be called when the competition between 2 competitors is processed to trigger a rating adjustment on the arena core for the competition's category
    pub fn trigger_rating_adjustment(
        &self,
        storage: &mut dyn Storage,
        category_id: Uint128,
        member_results: Vec<(MemberResult<Addr>, MemberResult<Addr>)>,
    ) -> Result<SubMsg, CompetitionError> {
        // Ensure Module has an owner
        let ownership = get_ownership(storage)?;
        let arena_core = ownership.owner.ok_or(CompetitionError::OwnershipError(
            cw_ownable::OwnershipError::NoOwner,
        ))?;

        Ok(SubMsg::reply_on_error(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: arena_core.to_string(),
                msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                    msg: arena_interface::core::ExecuteExt::AdjustRatings {
                        category_id,
                        member_results: member_results
                            .into_iter()
                            .map(|(member_result_1, member_result_2)| {
                                (member_result_1.into(), member_result_2.into())
                            })
                            .collect(),
                    },
                })?,
                funds: vec![],
            }),
            UPDATE_RATING_FAILED_REPLY_ID,
        ))
    }

    pub fn execute_update_stat_types(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        competition_id: Uint128,
        to_add: Vec<StatType>,
        to_remove: Vec<String>,
    ) -> Result<Response, CompetitionError> {
        // Check if the competition exists and the sender is authorized
        let competition = self
            .competitions
            .load(deps.storage, competition_id.u128())?;
        self.inner_validate_auth(&info.sender, &competition)?;

        // Add new stat types
        for stat_type in to_add {
            if self
                .stat_types
                .has(deps.storage, (competition_id.u128(), &stat_type.name))
            {
                return Err(CompetitionError::StatTypeAlreadyExists {
                    name: stat_type.name,
                });
            }
            self.stat_types.save(
                deps.storage,
                (competition_id.u128(), &stat_type.name),
                &stat_type,
            )?;
        }

        // Remove stat types
        for name in to_remove {
            if !self
                .stat_types
                .has(deps.storage, (competition_id.u128(), &name))
            {
                return Err(CompetitionError::StatTypeNotFound { name });
            }
            self.stat_types
                .remove(deps.storage, (competition_id.u128(), &name));
        }

        Ok(Response::new()
            .add_attribute("action", "update_stat_types")
            .add_attribute("competition_id", competition_id.to_string()))
    }

    pub fn execute_update_stats(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        competition_id: Uint128,
        updates: Vec<MemberStatUpdate>,
    ) -> Result<Response, CompetitionError> {
        // Check if the competition exists and the sender is authorized
        let competition = self
            .competitions
            .load(deps.storage, competition_id.u128())?;
        self.inner_validate_auth(&info.sender, &competition)?;

        for update in updates {
            let addr = deps.api.addr_validate(&update.addr)?;
            for stat in update.stats {
                // Check if the stat type exists
                if !self
                    .stat_types
                    .has(deps.storage, (competition_id.u128(), &stat.name))
                {
                    return Err(CompetitionError::StatTypeNotFound {
                        name: stat.name.clone(),
                    });
                }

                // Update the stat
                self.stats.save(
                    deps.storage,
                    (competition_id.u128(), &addr, &stat.name),
                    &stat.value,
                )?;
            }
        }

        Ok(Response::new()
            .add_attribute("action", "update_stats")
            .add_attribute("competition_id", competition_id.to_string()))
    }

    pub fn query(
        &self,
        deps: Deps,
        env: Env,
        msg: QueryBase<InstantiateExt, QueryExt, CompetitionExt>,
    ) -> StdResult<Binary> {
        match msg {
            QueryBase::Config {} => to_json_binary(&self.config.load(deps.storage)?),
            QueryBase::Competition { competition_id } => {
                to_json_binary(&self.query_competition(deps, env, competition_id)?)
            }
            QueryBase::DAO {} => to_json_binary(
                &self
                    .query_dao(deps)
                    .map_err(|x| StdError::generic_err(x.to_string()))?,
            ),
            QueryBase::Result { competition_id } => {
                to_json_binary(&self.query_result(deps, competition_id)?)
            }
            QueryBase::Evidence {
                competition_id,
                start_after,
                limit,
            } => to_json_binary(&self.query_evidence(deps, competition_id, start_after, limit)?),
            QueryBase::Competitions {
                start_after,
                limit,
                filter,
            } => to_json_binary(&self.query_competitions(deps, env, start_after, limit, filter)?),
            QueryBase::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
            QueryBase::CompetitionCount {} => {
                to_json_binary(&self.competition_count.load(deps.storage)?)
            }
            QueryBase::PaymentRegistry {} => to_json_binary(
                &self
                    .query_payment_registry(deps)
                    .map_err(|x| StdError::generic_err(x.to_string()))?,
            ),
            QueryBase::QueryExtension { .. } => Ok(Binary::default()),
            QueryBase::StatTypes { competition_id } => {
                to_json_binary(&self.query_stat_types(deps, competition_id)?)
            }
            QueryBase::Stats {
                competition_id,
                addr,
            } => to_json_binary(&self.query_stats(deps, competition_id, addr)?),
            QueryBase::_Phantom(_) => Ok(Binary::default()),
        }
    }

    pub fn query_stat_types(
        &self,
        deps: Deps,
        competition_id: Uint128,
    ) -> StdResult<Vec<StatType>> {
        let stat_types: Vec<StatType> = self
            .stat_types
            .prefix(competition_id.u128())
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| item.map(|(_, stat_type)| stat_type))
            .collect::<StdResult<Vec<_>>>()?;

        Ok(stat_types)
    }

    pub fn query_stats(
        &self,
        deps: Deps,
        competition_id: Uint128,
        addr: String,
    ) -> StdResult<Vec<StatMsg>> {
        let addr = deps.api.addr_validate(&addr)?;

        let stats: Vec<StatMsg> = self
            .stats
            .prefix((competition_id.u128(), &addr))
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| item.map(|(key, value)| StatMsg { name: key, value }))
            .collect::<StdResult<Vec<_>>>()?;

        Ok(stats)
    }

    pub fn query_is_dao_member(&self, deps: Deps, addr: &Addr, height: u64) -> bool {
        let result = self.query_dao(deps);

        if let Ok(dao) = result {
            let result = deps
                .querier
                .query_wasm_smart::<dao_interface::voting::VotingPowerAtHeightResponse>(
                    &dao,
                    &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                        address: addr.to_string(),
                        height: Some(height),
                    },
                );

            if let Ok(voting_power) = result {
                return !voting_power.power.is_zero();
            }
        }

        false
    }

    pub fn query_payment_registry(&self, deps: Deps) -> Result<Option<String>, CompetitionError> {
        let owner = get_ownership(deps.storage)?
            .owner
            .ok_or(CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NoOwner,
            ))?;

        let payment_registry: Option<String> = deps.querier.query_wasm_smart(
            owner,
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: arena_interface::core::QueryExt::PaymentRegistry {},
            },
        )?;

        Ok(payment_registry)
    }

    pub fn query_result(
        &self,
        deps: Deps,
        competition_id: Uint128,
    ) -> StdResult<Option<Distribution<Addr>>> {
        self.competition_result
            .load(deps.storage, competition_id.u128())
    }

    pub fn query_evidence(
        &self,
        deps: Deps,
        competition_id: Uint128,
        start_after: Option<Uint128>,
        limit: Option<u32>,
    ) -> StdResult<Vec<Evidence>> {
        let start_after_bound = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(30).max(30);

        self.competition_evidence
            .prefix(competition_id.u128())
            .range(
                deps.storage,
                start_after_bound,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .map(|x| x.map(|y| y.1))
            .take(limit as usize)
            .collect::<StdResult<Vec<_>>>()
    }

    pub fn query_competition(
        &self,
        deps: Deps,
        env: Env,
        competition_id: Uint128,
    ) -> StdResult<CompetitionResponse<CompetitionExt>> {
        let rules = self
            .competition_rules
            .may_load(deps.storage, competition_id.u128())?;

        Ok(self
            .competitions
            .load(deps.storage, competition_id.u128())?
            .into_response(rules, &env.block))
    }

    pub fn query_arena_tax_config(
        &self,
        deps: Deps,
        height: u64,
    ) -> Result<TaxConfigurationResponse, CompetitionError> {
        let owner = get_ownership(deps.storage)?
            .owner
            .ok_or(CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NoOwner,
            ))?;

        deps.querier
            .query_wasm_smart(
                owner,
                &arena_interface::core::QueryMsg::QueryExtension {
                    msg: arena_interface::core::QueryExt::TaxConfig { height },
                },
            )
            .map_err(Into::into)
    }

    pub fn query_dao(&self, deps: Deps) -> Result<Addr, cw_ownable::OwnershipError> {
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
        let limit = limit.unwrap_or(30).max(30);

        match filter {
            None => cw_paginate::paginate_indexed_map(
                &self.competitions,
                deps.storage,
                start_after_bound,
                Some(limit),
                |x, y| {
                    let rules = self.competition_rules.may_load(deps.storage, x)?;
                    Ok(y.into_response(rules, &env.block))
                },
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
                        cosmwasm_std::Order::Descending,
                    )
                    .flat_map(|x| {
                        x.map(|y| {
                            let rules = self.competition_rules.may_load(deps.storage, y.0)?;
                            Ok(y.1.into_response(rules, &env.block))
                        })
                    })
                    .take(limit as usize)
                    .collect::<StdResult<Vec<_>>>(),
                CompetitionsFilter::Category { id } => self
                    .competitions
                    .idx
                    .category
                    .prefix(id.unwrap_or(Uint128::zero()).u128())
                    .range(
                        deps.storage,
                        start_after_bound,
                        None,
                        cosmwasm_std::Order::Descending,
                    )
                    .flat_map(|x| {
                        x.map(|y| {
                            let rules = self.competition_rules.may_load(deps.storage, y.0)?;
                            Ok(y.1.into_response(rules, &env.block))
                        })
                    })
                    .take(limit as usize)
                    .collect::<StdResult<Vec<_>>>(),
                CompetitionsFilter::Host(addr) => self
                    .competitions
                    .idx
                    .host
                    .prefix(addr)
                    .range(
                        deps.storage,
                        start_after_bound,
                        None,
                        cosmwasm_std::Order::Descending,
                    )
                    .flat_map(|x| {
                        x.map(|y| {
                            let rules = self.competition_rules.may_load(deps.storage, y.0)?;
                            Ok(y.1.into_response(rules, &env.block))
                        })
                    })
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
            UPDATE_RATING_FAILED_REPLY_ID => self.reply_update_rating_failed(deps, msg),
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
                    None => Err(CompetitionError::UnknownCompetitionId {
                        id: Uint128::new(id),
                    }),
                }
            })?;

        Ok(Response::new().add_attribute("action", "reply_process"))
    }

    pub fn reply_update_rating_failed(
        &self,
        _deps: DepsMut,
        _msg: Reply,
    ) -> Result<Response, CompetitionError> {
        // There should be an event if the rating update has failed, but it should not cause the overall message to fail
        Ok(Response::new().add_attribute("action", "update_rating_failed"))
    }

    pub fn migrate_from_v1_6_to_v1_7(&self, deps: DepsMut) -> Result<(), CompetitionError> {
        // Index for competition categories has changed, so we need to update the indexes here
        // Not too many, so we can just do it in the migration
        let competition_range = self
            .competitions
            .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
            .collect::<StdResult<Vec<_>>>()?;

        for (competition_id, competition) in competition_range {
            self.competitions.replace(
                deps.storage,
                competition_id,
                Some(&competition),
                Some(&competition),
            )?;
        }

        Ok(())
    }

    pub fn migrate_from_v1_8_2_to_v2(
        &self,
        deps: DepsMut,
        env: Env,
    ) -> Result<(), CompetitionError> {
        // Competition status 'Active' now stores its height for the payment registry
        // Not too many, so we can just do it in the migration
        let competition_range = self
            .competitions
            .idx
            .status
            .prefix(CompetitionStatus::Active { height: 0u64 }.to_string())
            .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
            .collect::<StdResult<Vec<_>>>()?;

        for (competition_id, competition) in competition_range {
            let new_competition = Competition {
                status: CompetitionStatus::Active {
                    height: env.block.height,
                },
                ..competition.clone()
            };

            self.competitions.replace(
                deps.storage,
                competition_id,
                Some(&new_competition),
                Some(&competition),
            )?;
        }

        Ok(())
    }
}
