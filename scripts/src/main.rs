pub use arena::*;
use cw_orch::{anyhow, prelude::*};
pub use dao_dao::*;
use orch_interface::{
    arena_competition_enrollment::ArenaCompetitionEnrollmentContract,
    arena_core::ArenaCoreContract, arena_discord_identity::ArenaDiscordIdentityContract,
    arena_escrow::ArenaEscrowContract, arena_group::ArenaGroupContract,
    arena_league_module::ArenaLeagueModuleContract,
    arena_payment_registry::ArenaPaymentRegistryContract,
    arena_tournament_module::ArenaTournamentModuleContract,
    arena_wager_module::ArenaWagerModuleContract, dao_dao_core::DaoDaoCoreContract,
};
use std::env;

mod arena;
mod dao_dao;
#[cfg(test)]
mod tests;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    dotenv::from_filename(".env.keys").ok();
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();
    let command = parse_command(&args);

    match command {
        Command::Deploy(network, component) => deploy(network, component)?,
        Command::Unknown => {
            println!(
                "Usage: deploy <network: testnet|mainnet|all> <component: all|core|dao_core|...>"
            );
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Command {
    Deploy(Network, DeployComponent),
    Unknown,
}

#[derive(Debug)]
enum Network {
    Testnet,
    Mainnet,
    All,
}

#[derive(Debug)]
enum DeployComponent {
    All,
    Core,
    Tournament,
    Enrollment,
    CompetitionModules,
    Group,
    Identity,
    DaoCore,
    Registry,
    Escrow,
}

impl Network {
    fn parse(input: &str) -> Option<Self> {
        match input {
            "testnet" => Some(Self::Testnet),
            "mainnet" => Some(Self::Mainnet),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

impl DeployComponent {
    fn parse(input: &str) -> Option<Self> {
        match input {
            "all" => Some(Self::All),
            "core" => Some(Self::Core),
            "dao_core" => Some(Self::DaoCore),
            "tournament" => Some(Self::Tournament),
            "enrollment" => Some(Self::Enrollment),
            "competition_modules" => Some(Self::CompetitionModules),
            "group" => Some(Self::Group),
            "identity" => Some(Self::Identity),
            "registry" => Some(Self::Registry),
            "escrow" => Some(Self::Escrow),
            _ => None,
        }
    }
}

fn parse_command(args: &[String]) -> Command {
    if args.len() < 4 || args[1] != "deploy" {
        return Command::Unknown;
    }

    let network = Network::parse(&args[2]);
    let component = DeployComponent::parse(&args[3]);

    match (network, component) {
        (Some(network), Some(component)) => Command::Deploy(network, component),
        _ => Command::Unknown,
    }
}

fn deploy(network: Network, component: DeployComponent) -> anyhow::Result<()> {
    match network {
        Network::All => {
            deploy_to_network(Network::Testnet, &component)?;
            deploy_to_network(Network::Mainnet, &component)?;
        }
        _ => deploy_to_network(network, &component)?,
    }

    Ok(())
}

fn deploy_to_network(network: Network, component: &DeployComponent) -> anyhow::Result<()> {
    let daemon = match network {
        Network::Testnet => Daemon::builder(cw_orch::daemon::networks::PION_1).build()?,
        Network::Mainnet => Daemon::builder(cw_orch::daemon::networks::NEUTRON_1).build()?,
        Network::All => unreachable!("'All' should not reach here."),
    };

    match component {
        DeployComponent::All => {
            deploy_core(&daemon)?;
            deploy_tournament(&daemon)?;
            deploy_enrollment(&daemon)?;
            deploy_competition_modules(&daemon)?;
            deploy_group(&daemon)?;
            if matches!(network, Network::Mainnet) {
                deploy_identity(&daemon)?;
            }
            deploy_registry(&daemon)?;
            deploy_escrow(&daemon)?;
        }
        DeployComponent::Core => deploy_core(&daemon)?,
        DeployComponent::DaoCore => deploy_dao_core(&daemon)?,
        DeployComponent::Tournament => deploy_tournament(&daemon)?,
        DeployComponent::Enrollment => deploy_enrollment(&daemon)?,
        DeployComponent::CompetitionModules => deploy_competition_modules(&daemon)?,
        DeployComponent::Group => deploy_group(&daemon)?,
        DeployComponent::Identity => deploy_identity(&daemon)?,
        DeployComponent::Registry => deploy_registry(&daemon)?,
        DeployComponent::Escrow => deploy_escrow(&daemon)?,
    }

    Ok(())
}

// Deployment Functions
fn deploy_core(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaCoreContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_dao_core(daemon: &Daemon) -> anyhow::Result<()> {
    DaoDaoCoreContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_tournament(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaTournamentModuleContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_enrollment(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaCompetitionEnrollmentContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_competition_modules(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaWagerModuleContract::new(daemon.clone()).upload()?;
    ArenaLeagueModuleContract::new(daemon.clone()).upload()?;
    ArenaTournamentModuleContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_group(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaGroupContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_identity(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaDiscordIdentityContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_registry(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaPaymentRegistryContract::new(daemon.clone()).upload()?;
    Ok(())
}

fn deploy_escrow(daemon: &Daemon) -> anyhow::Result<()> {
    ArenaEscrowContract::new(daemon.clone()).upload()?;
    Ok(())
}
