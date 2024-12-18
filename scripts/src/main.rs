use arena::Arena;
use cw_orch::{anyhow, prelude::*};
use orch_interface::{
    arena_competition_enrollment::ArenaCompetitionEnrollmentContract,
    arena_core::ArenaCoreContract, arena_discord_identity::ArenaDiscordIdentityContract,
    arena_group::ArenaGroupContract, arena_league_module::ArenaLeagueModuleContract,
    arena_token_gateway::ArenaTokenGatewayContract,
    arena_tournament_module::ArenaTournamentModuleContract,
    arena_wager_module::ArenaWagerModuleContract, cw_abc::CwAbcContract,
    cw_payroll_factory::DaoPayrollFactory,
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
    TokenGateway,
    CompetitionModules,
    PayrollFactory,
    Abc,
    Group,
    Identity,
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
        "token_gateway" => DeployComponent::TokenGateway,
        "competition_modules" => DeployComponent::CompetitionModules,
        "payroll_factory" => DeployComponent::PayrollFactory,
        "abc" => DeployComponent::Abc,
        "group" => DeployComponent::Group,
        "identity" => DeployComponent::Identity,
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
        DeployComponent::TokenGateway => deploy_token_gateway(daemon)?,
        DeployComponent::CompetitionModules => deploy_competition_modules(daemon)?,
        DeployComponent::PayrollFactory => deploy_payroll_factory(daemon)?,
        DeployComponent::Abc => deploy_cw_abc(daemon)?,
        DeployComponent::Group => deploy_group(daemon)?,
        DeployComponent::Identity => deploy_identity(daemon)?,
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

fn deploy_token_gateway(daemon: Daemon) -> anyhow::Result<()> {
    let token_gateway = ArenaTokenGatewayContract::new(daemon);
    token_gateway.upload()?;
    Ok(())
}

fn deploy_competition_modules(daemon: Daemon) -> anyhow::Result<()> {
    let wager_module = ArenaWagerModuleContract::new(daemon.clone());
    wager_module.upload()?;
    let league_module = ArenaLeagueModuleContract::new(daemon.clone());
    league_module.upload()?;
    let tournament_module = ArenaTournamentModuleContract::new(daemon);
    tournament_module.upload()?;
    Ok(())
}

fn deploy_payroll_factory(daemon: Daemon) -> anyhow::Result<()> {
    let payroll_factory = DaoPayrollFactory::new(daemon);
    payroll_factory.upload()?;
    Ok(())
}

fn deploy_cw_abc(daemon: Daemon) -> anyhow::Result<()> {
    let cw_abc = CwAbcContract::new(daemon);
    cw_abc.upload()?;
    Ok(())
}

fn deploy_group(daemon: Daemon) -> anyhow::Result<()> {
    let group = ArenaGroupContract::new(daemon);
    group.upload()?;
    Ok(())
}

fn deploy_identity(daemon: Daemon) -> anyhow::Result<()> {
    let identity = ArenaDiscordIdentityContract::new(daemon);
    identity.upload()?;
    Ok(())
}

mod arena;
mod dao_dao;
#[cfg(test)]
mod tests;
