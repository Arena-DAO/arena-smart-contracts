pub mod arena_competition_enrollment;
pub mod arena_core;
pub mod arena_league_module;
pub mod arena_payment_registry;
#[cfg(feature = "abc")]
pub mod arena_token_gateway;
pub mod arena_tournament_module;
pub mod arena_wager_module;
mod deploy;
mod helpers;

pub(crate) const PREFIX: &str = "arena";
pub(crate) const ADMIN: &str = "ismellike";
pub(crate) const DENOM: &str = "USDC";
