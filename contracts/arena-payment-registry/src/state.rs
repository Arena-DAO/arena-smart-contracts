use cosmwasm_std::Addr;
use cw_balance::Distribution;
use cw_storage_plus::{SnapshotMap, Strategy};

pub const PRESET_DISTRIBUTIONS: SnapshotMap<&Addr, Distribution<Addr>> = SnapshotMap::new(
    "preset_distributions",
    "preset_distributions__check",
    "preset_distributions__change",
    Strategy::EveryBlock {},
);
