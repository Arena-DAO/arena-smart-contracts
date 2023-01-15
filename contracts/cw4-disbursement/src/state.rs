use cosmwasm_std::Addr;
use cw_disbursement::DisbursementData;
use cw_storage_plus::{Item, Map};

//maps a key to this contract's disbursement distribution data
pub const DISBURSEMENT_DATA: Map<String, DisbursementData> = Map::new("disbursement_data");
pub const DAO: Item<Addr> = Item::new("dao");
