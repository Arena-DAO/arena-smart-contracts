use arena::Arena;
use cw_orch::prelude::*;
use orch_interface::{
    arena_competition_enrollment::ArenaCompetitionEnrollmentContract,
    arena_core::ArenaCoreContract, arena_tournament_module::ArenaTournamentModuleContract,
};
use std::env;

#[allow(clippy::collapsible_if)]
fn main() {
    let args: Vec<String> = env::args().collect();

    dotenv::dotenv().ok(); // Used to load the `.env` file if any
    dotenv::from_filename(".env.keys").ok();
    pretty_env_logger::init();

    if args.contains(&"deploy".to_string()) {
        if args.contains(&"testnet".to_string()) {
            // We start by creating a daemon. This daemon will be used to interact with the chain.
            let daemon = Daemon::builder()
                // set the network to use
                .chain(cw_orch::daemon::networks::PION_1) // chain parameter
                .build()
                .unwrap();

            if args.contains(&"all".to_string()) {
                deploy_daemon(daemon);
            } else if args.contains(&"tournament".to_string()) {
                deploy_tournament(daemon);
            } else if args.contains(&"enrollment".to_string()) {
                deploy_enrollment(daemon);
            }
        }
        if args.contains(&"mainnet".to_string()) {
            // We start by creating a daemon. This daemon will be used to interact with the chain.
            let daemon = Daemon::builder()
                // set the network to use
                .chain(cw_orch::daemon::networks::NEUTRON_1) // chain parameter
                .build()
                .unwrap();

            if args.contains(&"all".to_string()) {
                deploy_daemon(daemon);
            } else if args.contains(&"core".to_string()) {
                deploy_core(daemon);
            }
        }
    }
}

fn deploy_daemon(daemon: Daemon) {
    let arena = Arena::new(daemon);

    let upload_res = arena.upload(false);
    assert!(upload_res.is_ok());
}

fn deploy_core(daemon: Daemon) {
    let core = ArenaCoreContract::new(daemon);

    core.upload().ok();
}

fn deploy_tournament(daemon: Daemon) {
    let tournament = ArenaTournamentModuleContract::new(daemon);

    tournament.upload().ok();
}

fn deploy_enrollment(daemon: Daemon) {
    let enrollment = ArenaCompetitionEnrollmentContract::new(daemon);

    enrollment.upload().ok();
}

mod arena;
mod dao_dao;
#[cfg(test)]
mod tests;
