use cosmwasm_std::{DepsMut, Env, Order, StdResult};
use cw_utils::Expiration;

use crate::{
    state::{
        enrollment_entries, CompetitionInfo, EnrollmentEntry, LegacyCompetitionInfo,
        LEGACY_ENROLLMENTS,
    },
    ContractError,
};

pub fn migrate_from_v2_3_to_v2_3_1(deps: DepsMut, env: Env) -> Result<(), ContractError> {
    for (enrollment_id, enrollment) in LEGACY_ENROLLMENTS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        let competition_info = match enrollment.competition_info {
            LegacyCompetitionInfo::Pending {
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                additional_layered_fees,
                escrow,
                group_contract,
            } => {
                let date = match expiration {
                    Expiration::AtTime(date) => date,
                    _ => env.block.time,
                };

                CompetitionInfo::Pending {
                    name,
                    description,
                    date,
                    duration: 3600,
                    rules,
                    rulesets,
                    banner,
                    additional_layered_fees,
                    escrow,
                    group_contract,
                }
            }
            LegacyCompetitionInfo::Existing { id } => CompetitionInfo::Existing { id },
        };

        let new_enrollment = EnrollmentEntry {
            min_members: enrollment.min_members,
            max_members: enrollment.max_members,
            entry_fee: enrollment.entry_fee,
            duration_before: 86400,
            has_finalized: enrollment.has_finalized,
            competition_info,
            competition_type: enrollment.competition_type,
            host: enrollment.host,
            category_id: enrollment.category_id,
            competition_module: enrollment.competition_module,
            required_team_size: enrollment.required_team_size,
            use_dao_host: None,
        };

        enrollment_entries().replace(
            deps.storage,
            enrollment_id,
            Some(&new_enrollment),
            Some(&new_enrollment),
        )?;
    }

    Ok(())
}
