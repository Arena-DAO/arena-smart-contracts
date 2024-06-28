use std::marker::PhantomData;

#[allow(unused_imports)]
use crate::competition::state::{CompetitionResponse, CompetitionStatus, Config, Evidence};
use crate::fees::FeeInformation;
use cosmwasm_schema::{cw_serde, schemars::JsonSchema, QueryResponses};
use cosmwasm_std::{Binary, Deps, StdResult, Uint128};
use cw_balance::Distribution;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use cw_utils::Expiration;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cw_serde]
pub struct InstantiateBase<InstantiateExt> {
    pub key: String, //this is used to map a key (wager, tournament, league) to a module
    pub description: String,
    pub extension: InstantiateExt,
}

#[cw_ownable_execute]
#[cw_serde]
#[allow(clippy::large_enum_variant)]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteBase<ExecuteExt, CompetitionInstantiateExt> {
    #[cw_orch(payable)]
    JailCompetition {
        competition_id: Uint128,
        title: String,
        description: String,
        distribution: Option<Distribution<String>>,
        additional_layered_fees: Option<FeeInformation<String>>,
    },
    ActivateCompetition {},
    AddCompetitionHook {
        competition_id: Uint128,
    },
    RemoveCompetitionHook {
        competition_id: Uint128,
    },
    ExecuteCompetitionHook {
        competition_id: Uint128,
        distribution: Option<Distribution<String>>,
    },
    CreateCompetition {
        /// The competition's host
        /// Defaults to info.sender
        /// This can only be overridden by valid competition enrollment modules
        host: Option<String>,
        category_id: Option<Uint128>,
        escrow: Option<EscrowInstantiateInfo>,
        name: String,
        description: String,
        expiration: Expiration,
        rules: Vec<String>,
        rulesets: Vec<Uint128>,
        banner: Option<String>,
        /// Determines if the competition is automatically activated if all dues are paid
        /// Defaults to true
        should_activate_on_funded: Option<bool>,
        instantiate_extension: CompetitionInstantiateExt,
    },
    SubmitEvidence {
        competition_id: Uint128,
        evidence: Vec<String>,
    },
    ProcessCompetition {
        competition_id: Uint128,
        distribution: Option<Distribution<String>>,
    },
    Extension {
        msg: ExecuteExt,
    },
    ActivateCompetitionManually {
        id: Uint128,
    },
    MigrateEscrows {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        filter: Option<CompetitionsFilter>,
        escrow_code_id: u64,
        escrow_migrate_msg: crate::escrow::MigrateMsg,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryBase<InstantiateExt, QueryExt, CompetitionExt>
where
    InstantiateExt: Serialize + std::fmt::Debug + DeserializeOwned,
    QueryExt: JsonSchema,
    CompetitionExt: Serialize + std::fmt::Debug + DeserializeOwned,
{
    #[returns(Config<InstantiateExt>)]
    Config {},
    #[returns(String)]
    DAO {},
    #[returns(Uint128)]
    CompetitionCount {},
    #[returns(CompetitionResponse<CompetitionExt>)]
    Competition { competition_id: Uint128 },
    #[returns(Vec<CompetitionResponse<CompetitionExt>>)]
    Competitions {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        filter: Option<CompetitionsFilter>,
    },
    #[returns(Vec<Evidence>)]
    Evidence {
        competition_id: Uint128,
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },
    #[returns(Option<Distribution<String>>)]
    Result { competition_id: Uint128 },
    #[returns(cosmwasm_std::Binary)]
    QueryExtension { msg: QueryExt },
    #[serde(skip)]
    #[returns(PhantomData<(InstantiateExt, CompetitionExt)>)]
    _Phantom(PhantomData<(InstantiateExt, CompetitionExt)>),
}

#[cw_serde]
pub struct EscrowInstantiateInfo {
    /// Code ID of the contract to be instantiated.
    pub code_id: u64,
    /// Instantiate message to be used to create the contract.
    pub msg: Binary,
    /// Label for the instantiated contract.
    pub label: String,
    /// Optional additional layered fees
    pub additional_layered_fees: Option<Vec<FeeInformation<String>>>,
}

#[cw_serde]
pub enum CompetitionsFilter {
    CompetitionStatus { status: CompetitionStatus },
    Category { id: Option<Uint128> },
    Host(String),
}

#[cw_serde]
pub enum HookDirection {
    Incoming,
    Outgoing,
}
pub trait ToCompetitionExt<T> {
    fn to_competition_ext(&self, deps: Deps) -> StdResult<T>;
}
