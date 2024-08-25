use crate::tests::deploy::ArenaDeployData;
use crate::Arena;
use cw_orch::{anyhow, prelude::*};

pub fn setup_arena(mock: &MockBech32) -> anyhow::Result<(Arena<MockBech32>, Addr)> {
    let admin = mock.addr_make(crate::tests::ADMIN);
    let arena = Arena::deploy_on(
        mock.clone(),
        ArenaDeployData {
            admin: admin.clone(),
            voting_module_override: None,
        },
    )?;
    mock.next_block()?;
    Ok((arena, admin))
}
