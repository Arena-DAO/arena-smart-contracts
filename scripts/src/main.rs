use arena::Arena;
use cw_orch::{anyhow, prelude::*};
use orch_interface::{
    arena_competition_enrollment::ArenaCompetitionEnrollmentContract,
    arena_core::ArenaCoreContract, arena_tournament_module::ArenaTournamentModuleContract,
};
use std::env;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    dotenv::from_filename(".env.keys").ok();
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();
    let command = parse_command(&args);

    match command {
        Command::Deploy(network, component) => deploy(network, component)?,
        Command::Unknown => println!("Unknown command. Use 'deploy <network> <component>'."),
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
}

#[derive(Debug)]
enum DeployComponent {
    All,
    Core,
    Tournament,
    Enrollment,
}

fn parse_command(args: &[String]) -> Command {
    if args.len() < 4 || args[1] != "deploy" {
        return Command::Unknown;
    }

    let network = match args[2].as_str() {
        "testnet" => Network::Testnet,
        "mainnet" => Network::Mainnet,
        _ => return Command::Unknown,
    };

    let component = match args[3].as_str() {
        "all" => DeployComponent::All,
        "core" => DeployComponent::Core,
        "tournament" => DeployComponent::Tournament,
        "enrollment" => DeployComponent::Enrollment,
        _ => return Command::Unknown,
    };

    Command::Deploy(network, component)
}

fn deploy(network: Network, component: DeployComponent) -> anyhow::Result<()> {
    let daemon = match network {
        Network::Testnet => Daemon::builder(cw_orch::daemon::networks::PION_1).build()?,
        Network::Mainnet => Daemon::builder(cw_orch::daemon::networks::NEUTRON_1).build()?,
    };

    match component {
        DeployComponent::All => deploy_all(daemon)?,
        DeployComponent::Core => deploy_core(daemon)?,
        DeployComponent::Tournament => deploy_tournament(daemon)?,
        DeployComponent::Enrollment => deploy_enrollment(daemon)?,
    }

    Ok(())
}

fn deploy_all(daemon: Daemon) -> anyhow::Result<()> {
    let arena = Arena::new(daemon);
    arena.upload(false)?;
    Ok(())
}

fn deploy_core(daemon: Daemon) -> anyhow::Result<()> {
    let core = ArenaCoreContract::new(daemon);
    core.upload()?;
    Ok(())
}

fn deploy_tournament(daemon: Daemon) -> anyhow::Result<()> {
    let tournament = ArenaTournamentModuleContract::new(daemon);
    tournament.upload()?;
    Ok(())
}

fn deploy_enrollment(daemon: Daemon) -> anyhow::Result<()> {
    let enrollment = ArenaCompetitionEnrollmentContract::new(daemon);
    enrollment.upload()?;
    Ok(())
}

mod arena;
mod dao_dao;
#[cfg(test)]
mod tests;
