use std::marker::PhantomData;

use arena_interface::{
    competition::{
        msg::{
            CompetitionsFilter, EscrowInstantiateInfo, ExecuteBase, HookDirection, InstantiateBase,
            ModuleInfo, QueryBase, ToCompetitionExt,
        },
        state::{
            Competition, CompetitionListItemResponse, CompetitionResponse, CompetitionStatus,
            Config, Evidence,
        },
    },
    core::{CompetitionModuleResponse, ProposeMessage, TaxConfigurationResponse},
    fees::FeeInformation,
    ratings::MemberResult,
};
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    ensure, instantiate2_address, to_json_binary, Addr, Binary, CosmosMsg, Decimal,
    DecimalRangeExceeded, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdError,
    StdResult, Storage, SubMsg, Uint128, WasmMsg,
};
use cw_balance::Distribution;
use cw_ownable::{get_ownership, initialize_owner};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::CompetitionError;

pub const PROCESS_REPLY_ID: u64 = 1;
pub const UPDATE_RATING_FAILED_REPLY_ID: u64 = 2;

pub struct CompetitionIndexes<'a, CompetitionExt> {
    pub status: MultiIndex<'a, String, Competition<CompetitionExt>, u128>,
    pub category: MultiIndex<'a, String, Competition<CompetitionExt>, u128>,
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
                |_x, d: &Competition<CompetitionExt>| format!("{:?}", d.category_id),
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
            ExecuteBase::JailCompetition { propose_message } => {
                self.execute_jail_competition(deps, env, info, propose_message)
            }
            ExecuteBase::CreateCompetition {
                category_id,
                host,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                should_activate_on_funded,
                instantiate_extension,
            } => self.execute_create_competition(
                &mut deps,
                &env,
                category_id,
                host,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                should_activate_on_funded,
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
            ExecuteBase::Activate {} => self.execute_activate_from_escrow(deps, info),
            ExecuteBase::ActivateManually { id } => self.execute_activate_by_host(deps, info, id),
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
            ExecuteBase::ExecuteCompetitionHook {
                competition_id: _,
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
        let mut competition = self.competitions.may_load(deps.storage, id)?.ok_or(
            CompetitionError::UnknownCompetitionId {
                id: Uint128::new(id),
            },
        )?;

        // Update competition status
        competition.status = CompetitionStatus::Active;
        self.competitions.save(deps.storage, id, &competition)?;

        // Do not expect another activation message
        self.escrows_to_competitions
            .remove(deps.storage, &info.sender);

        Ok(Response::new()
            .add_attribute("id", id.to_string())
            .add_attribute("action", "activate"))
    }

    pub fn execute_activate_by_host(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: Uint128,
    ) -> Result<Response, CompetitionError> {
        let mut competition = self
            .competitions
            .may_load(deps.storage, id.u128())?
            .ok_or(CompetitionError::UnknownCompetitionId { id })?;

        ensure!(
            competition.status == CompetitionStatus::Pending,
            CompetitionError::InvalidCompetitionStatus {
                current_status: competition.status
            }
        );
        ensure!(
            info.sender == competition.host || info.sender == competition.admin_dao,
            CompetitionError::Unauthorized {}
        );

        // Only allow activation by host if the escrow should not activate on funded
        let mut msgs = vec![];
        if let Some(escrow) = &competition.escrow {
            if deps.querier.query_wasm_smart::<bool>(
                escrow.to_string(),
                &arena_interface::escrow::QueryMsg::ShouldActivateOnFunded {},
            )? {
                return Err(CompetitionError::Unauthorized {});
            }

            // Trigger the activation on the escrow
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: escrow.to_string(),
                msg: to_json_binary(&arena_interface::escrow::ExecuteMsg::Activate {})?,
                funds: vec![],
            }));
        } else {
            // Update competition status
            competition.status = CompetitionStatus::Active {};
            self.competitions
                .save(deps.storage, id.u128(), &competition)?;
        }

        Ok(Response::new()
            .add_attribute("id", id.to_string())
            .add_attribute("action", "activate_by_host")
            .add_messages(msgs))
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

        let id = propose_message.competition_id;

        // Update competition status
        self.competitions.update(deps.storage, id.u128(), |x| {
            let mut competition = x.ok_or(CompetitionError::UnknownCompetitionId { id })?;

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
            msg: to_json_binary(&arena_interface::core::ExecuteMsg::Propose {
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
        category_id: Option<Uint128>,
        host: ModuleInfo,
        escrow: Option<EscrowInstantiateInfo>,
        name: String,
        description: String,
        expiration: cw_utils::Expiration,
        rules: Vec<String>,
        rulesets: Vec<Uint128>,
        banner: Option<String>,
        should_activate_on_funded: Option<bool>,
        extension: &CompetitionInstantiateExt,
    ) -> Result<Response, CompetitionError> {
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

        // Setup
        let competition_id = self
            .competition_count
            .update(deps.storage, |x| -> StdResult<_> {
                Ok(x.checked_add(Uint128::one())?)
            })?;
        let admin_dao = self.query_dao(deps.as_ref())?;
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

        // Validate fees
        let fees = escrow
            .as_ref()
            .and_then(|x| {
                x.additional_layered_fees.as_ref().map(|y| {
                    y.iter()
                        .map(|z| z.into_checked(deps.as_ref()))
                        .collect::<StdResult<Vec<FeeInformation<Addr>>>>()
                })
            })
            .transpose()?;

        if let Some(ref fees) = fees {
            for fee in fees {
                ensure!(
                    fee.tax < Decimal::one(),
                    CompetitionError::DecimalRangeExceeded(DecimalRangeExceeded {})
                );
                ensure!(
                    !fee.tax.is_zero(),
                    CompetitionError::StdError(StdError::generic_err("Fee cannot be 0"))
                );
            }
        }
        // instantiate2 escrow
        let escrow_addr = match escrow {
            Some(info) => {
                let code_info = deps.querier.query_wasm_code_info(info.code_id)?;
                let canonical_addr =
                    instantiate2_address(&code_info.checksum, &canonical_creator, &salt)?;

                msgs.push(WasmMsg::Instantiate2 {
                    admin: Some(host_addr.to_string()),
                    code_id: info.code_id,
                    label: info.label.clone(),
                    msg: info.msg.clone(),
                    funds: vec![],
                    salt: salt.into(),
                });

                let addr = deps.api.addr_humanize(&canonical_addr)?;

                self.escrows_to_competitions
                    .save(deps.storage, &addr, &competition_id.u128())?;

                Some(addr)
            }
            None => {
                if should_activate_on_funded.unwrap_or(true) {
                    initial_status = CompetitionStatus::Active;
                }
                None
            }
        };

        if category_id.is_some() || !rulesets.is_empty() {
            // Validate that category and rulesets are valid
            let result: bool = deps.querier.query_wasm_smart(
                arena_core,
                &arena_interface::core::QueryMsg::QueryExtension {
                    msg: arena_interface::core::QueryExt::IsValidCategoryAndRulesets {
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
        }

        // Create competition
        let competition = Competition {
            id: competition_id,
            category_id,
            admin_dao: admin_dao.clone(),
            host: host_addr,
            start_height: env.block.height,
            escrow: escrow_addr,
            name,
            description,
            expiration,
            rulesets,
            status: initial_status,
            extension: extension.to_competition_ext(deps.as_ref())?,
            fees,
            banner,
        };

        self.competition_rules
            .save(deps.storage, competition_id.u128(), &rules)?;
        self.competitions
            .save(deps.storage, competition_id.u128(), &competition)?;

        Ok(Response::new()
            .add_attribute("action", "create_competition")
            .add_attribute("competition_id", competition_id)
            .add_attribute(
                "escrow_addr",
                competition
                    .escrow
                    .map(|x| x.to_string())
                    .unwrap_or("None".to_string()),
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
                storage: &mut dyn Storage,
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
            if let Some(sub_msg) = post_processing(deps.storage, &competition)? {
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
            CompetitionStatus::Active => {
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

            let sub_msg = SubMsg::reply_on_success(
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: escrow.to_string(),
                    msg: to_json_binary(&arena_interface::escrow::ExecuteMsg::Distribute {
                        distribution: distribution_msg,
                        layered_fees,
                    })?,
                    funds: vec![],
                }),
                PROCESS_REPLY_ID,
            );

            self.temp_competition
                .save(deps.storage, &competition.id.u128())?;

            msgs.push(sub_msg);
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
            QueryBase::QueryExtension { .. } => Ok(Binary::default()),
            QueryBase::_Phantom(_) => Ok(Binary::default()),
        }
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
        let limit = limit.unwrap_or(10).max(30);

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
            .load(deps.storage, competition_id.u128())?;

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
        let core = cw_ownable::get_ownership(deps.storage)?;
        if core.owner.is_none() {
            return Err(CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NoOwner,
            ));
        }

        Ok(deps.querier.query_wasm_smart(core.owner.unwrap(),
        &dao_pre_propose_base::msg::QueryMsg::<arena_interface::core::QueryExt>::QueryExtension
                {
                    msg: arena_interface::core::QueryExt::TaxConfig { height }
                })?
       )
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
    ) -> StdResult<Vec<CompetitionListItemResponse<CompetitionExt>>> {
        let start_after_bound = start_after.map(Bound::exclusive);
        let limit = limit.unwrap_or(10).max(30);

        match filter {
            None => cw_paginate::paginate_indexed_map(
                &self.competitions,
                deps.storage,
                start_after_bound,
                Some(limit),
                |_x, y| Ok(y.into_list_item_response(&env.block)),
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
                    .map(|x| x.map(|y| y.1.into_list_item_response(&env.block)))
                    .take(limit as usize)
                    .collect::<StdResult<Vec<_>>>(),
                CompetitionsFilter::Category { id } => self
                    .competitions
                    .idx
                    .category
                    .prefix(format!("{:?}", id))
                    .range(
                        deps.storage,
                        start_after_bound,
                        None,
                        cosmwasm_std::Order::Descending,
                    )
                    .map(|x| x.map(|y| y.1.into_list_item_response(&env.block)))
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
                    .map(|x| x.map(|y| y.1.into_list_item_response(&env.block)))
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
}
