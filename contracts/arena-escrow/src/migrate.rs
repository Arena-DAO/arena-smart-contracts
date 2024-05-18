use cosmwasm_std::{from_json, Decimal, DepsMut};

use crate::{
    state::{DEFERRED_FEES, HAS_DISTRIBUTED},
    ContractError,
};

#[allow(clippy::redundant_closure)]
pub fn from_v1_3_to_v_1_4(deps: DepsMut) -> Result<(), ContractError> {
    if let Some(has_distributed) = HAS_DISTRIBUTED.may_load(deps.storage)? {
        if !has_distributed {
            HAS_DISTRIBUTED.remove(deps.storage);
        }
    }

    let prev_key = "tax_at_withdrawal".as_bytes();
    if let Some(tax_at_withdrawal) = deps
        .storage
        .get(prev_key)
        .map(|x| from_json::<Decimal>(x))
        .transpose()?
    {
        DEFERRED_FEES.save(deps.storage, &vec![tax_at_withdrawal])?;
        deps.storage.remove(prev_key);
    }

    Ok(())
}
