use arena::Arena;
use cw_orch::prelude::*;
use orch_interface::arena_tournament_module::ArenaTournamentModuleContract;
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
                deploy_testnet(daemon);
            } else if args.contains(&"tournament".to_string()) {
                deploy_tournament(daemon);
            }
        }
    }
}

fn deploy_testnet(daemon: Daemon) {
    let arena = Arena::new(daemon);

    let upload_res = arena.upload(false);
    assert!(upload_res.is_ok());
}

fn deploy_tournament(daemon: Daemon) {
    let tournament = ArenaTournamentModuleContract::new(daemon);

    tournament.upload().ok();
}

mod arena;
mod dao_dao;
#[cfg(test)]
mod tests;
