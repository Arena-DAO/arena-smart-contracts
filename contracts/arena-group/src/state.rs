use cosmwasm_std::{Addr, Uint64};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

pub struct MemberIndexes<'a> {
    pub seed: MultiIndex<'a, u64, Uint64, &'a Addr>,
}

impl<'a> IndexList<Uint64> for MemberIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Uint64>> + '_> {
        let v: Vec<&dyn Index<Uint64>> = vec![&self.seed];
        Box::new(v.into_iter())
    }
}

pub const MEMBER_COUNT: Item<Uint64> = Item::new("member_count");
pub fn members<'a>() -> IndexedMap<'a, &'a Addr, Uint64, MemberIndexes<'a>> {
    let indexes = MemberIndexes {
        seed: MultiIndex::new(|_, d| d.u64(), "members", "members__seed"),
    };
    IndexedMap::new("members", indexes)
}
