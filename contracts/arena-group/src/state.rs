use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint64};
use cw_storage_plus::{Index, IndexList, IndexedSnapshotMap, MultiIndex, SnapshotItem};

#[cw_serde]
pub struct MemberData {
    pub seed: Uint64,
    pub power: Uint64,
}

pub struct MemberIndexes<'a> {
    pub seed: MultiIndex<'a, u64, MemberData, &'a Addr>,
}

impl IndexList<MemberData> for MemberIndexes<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<MemberData>> + '_> {
        let v: Vec<&dyn Index<MemberData>> = vec![&self.seed];
        Box::new(v.into_iter())
    }
}

pub const TOTAL_POWER: SnapshotItem<Uint64> = SnapshotItem::new(
    "total_power",
    "total_power__check",
    "total_power__change",
    cw_storage_plus::Strategy::EveryBlock,
);
pub const MEMBER_COUNT: SnapshotItem<Uint64> = SnapshotItem::new(
    "member_count",
    "member_count__check",
    "member_count__change",
    cw_storage_plus::Strategy::EveryBlock,
);
pub fn members<'a>() -> IndexedSnapshotMap<&'a Addr, MemberData, MemberIndexes<'a>> {
    let indexes = MemberIndexes {
        seed: MultiIndex::new(|_, d| d.seed.u64(), "members", "members__seed"),
    };
    IndexedSnapshotMap::new(
        "members",
        "members__check",
        "members__change",
        cw_storage_plus::Strategy::EveryBlock,
        indexes,
    )
}
