use arena_interface::competition::types::DaoConfig;
use arena_team_enrollments::{
    msg::{CategoryStatusMsg, ExecuteMsgFns as _, QueryMsgFns as _},
    state::{ApplicantStatus, EntryStatus, TeamDaoConfig},
};
use cosmwasm_std::Uint128;
use cw_orch::{anyhow, prelude::*};
use dao_voting::threshold::{PercentageThreshold, Threshold};

use crate::tests::helpers::setup_arena;

use super::PREFIX;

#[test]
fn test_create_entry() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make("user1");
    arena.arena_team_enrollments.set_sender(&user1);

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create an entry
    let res = arena.arena_team_enrollments.create_entry(
        dao_config,
        "A test team for the competition".to_string(),
        "Test Team".to_string(),
        Some(Uint128::new(1)),
    )?;

    // Check for successful creation event
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "create_entry")));

    mock.next_block()?;

    // Query the created entry
    let entry = arena.arena_team_enrollments.get_entry(1)?;
    assert_eq!(entry.team_entry.title, "Test Team");
    assert_eq!(
        entry.team_entry.description,
        "A test team for the competition"
    );
    assert_eq!(entry.team_entry.creator, user1);
    assert_eq!(entry.team_entry.status, EntryStatus::Open);
    assert_eq!(entry.team_entry.category_id, Some(Uint128::new(1)));

    Ok(())
}

#[test]
fn test_list_entries() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make("user1");
    let user2 = mock.addr_make("user2");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create multiple entries
    arena.arena_team_enrollments.set_sender(&user1);
    arena.arena_team_enrollments.create_entry(
        dao_config.clone(),
        "First team".to_string(),
        "Team Alpha".to_string(),
        Some(Uint128::new(1)),
    )?;

    arena.arena_team_enrollments.set_sender(&user2);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Second team".to_string(),
        "Team Beta".to_string(),
        Some(Uint128::new(2)),
    )?;

    mock.next_block()?;

    // List all entries
    let entries = arena
        .arena_team_enrollments
        .list_entries(None, None, None)?;
    assert_eq!(entries.len(), 2);

    // List entries by category and status
    let category_1_entries = arena.arena_team_enrollments.list_entries(
        Some(CategoryStatusMsg {
            category_id: Some(Uint128::new(1)),
            status: EntryStatus::Open,
        }),
        None,
        None,
    )?;
    assert_eq!(category_1_entries.len(), 1);
    assert_eq!(category_1_entries[0].team_entry.title, "Team Alpha");

    Ok(())
}

#[test]
fn test_update_entry_status() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let user1 = mock.addr_make("user1");
    let user2 = mock.addr_make("user2");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry as user1
    arena.arena_team_enrollments.set_sender(&user1);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    // Update status as creator (should succeed)
    let res = arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Closed)?;
    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "update_entry_status")));

    mock.next_block()?;

    // Verify status was updated
    let entry = arena.arena_team_enrollments.get_entry(1)?;
    assert_eq!(entry.team_entry.status, EntryStatus::Closed);

    // Try to update as non-creator (should fail)
    arena.arena_team_enrollments.set_sender(&user2);
    let result = arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Open);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_apply_to_entry() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant = mock.addr_make("applicant");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    // Apply as a different user
    arena.arena_team_enrollments.set_sender(&applicant);
    let res = arena.arena_team_enrollments.apply(1)?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "apply")));

    mock.next_block()?;

    // Query applicant status
    let applicant_response = arena
        .arena_team_enrollments
        .get_applicant(applicant.to_string(), 1)?;
    assert_eq!(applicant_response.applicant, applicant);
    assert_eq!(applicant_response.status, ApplicantStatus::Default);

    // Try to apply again (should fail)
    let result = arena.arena_team_enrollments.apply(1);
    assert!(result.is_err());

    // Try creator applying to own entry (should fail)
    arena.arena_team_enrollments.set_sender(&creator);
    let result = arena.arena_team_enrollments.apply(1);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_update_applicant_status() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant = mock.addr_make("applicant");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry and apply
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    arena.arena_team_enrollments.set_sender(&applicant);
    arena.arena_team_enrollments.apply(1)?;

    // Approve applicant as creator
    arena.arena_team_enrollments.set_sender(&creator);
    let res = arena.arena_team_enrollments.update_applicant_status(
        applicant.to_string(),
        1,
        ApplicantStatus::Approved,
    )?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "update_applicant_status")));

    mock.next_block()?;

    // Verify applicant is approved
    let applicant_response = arena
        .arena_team_enrollments
        .get_applicant(applicant.to_string(), 1)?;
    assert_eq!(applicant_response.status, ApplicantStatus::Approved);

    // Reject the applicant
    arena.arena_team_enrollments.update_applicant_status(
        applicant.to_string(),
        1,
        ApplicantStatus::Rejected {
            reason: "Not a good fit".to_string(),
        },
    )?;

    mock.next_block()?;

    // Verify applicant is rejected
    let applicant_response = arena
        .arena_team_enrollments
        .get_applicant(applicant.to_string(), 1)?;
    assert!(matches!(
        applicant_response.status,
        ApplicantStatus::Rejected { .. }
    ));

    Ok(())
}

#[test]
fn test_withdraw_application() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant = mock.addr_make("applicant");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry and apply
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Test Team".to_string(),
        "Description".to_string(),
        None,
    )?;

    arena.arena_team_enrollments.set_sender(&applicant);
    arena.arena_team_enrollments.apply(1)?;

    // Withdraw application
    let res = arena.arena_team_enrollments.withdraw_application(1)?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "withdraw_application")));

    mock.next_block()?;

    // Verify application is withdrawn (should not exist)
    let result = arena
        .arena_team_enrollments
        .get_applicant(applicant.to_string(), 1);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_list_applicants() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant1 = mock.addr_make("applicant1");
    let applicant2 = mock.addr_make("applicant2");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    // Apply with multiple users
    arena.arena_team_enrollments.set_sender(&applicant1);
    arena.arena_team_enrollments.apply(1)?;

    arena.arena_team_enrollments.set_sender(&applicant2);
    arena.arena_team_enrollments.apply(1)?;

    mock.next_block()?;

    // List all applicants
    let applicants = arena
        .arena_team_enrollments
        .list_applicants(1, None, None, None)?;
    assert_eq!(applicants.len(), 2);

    let applicant_addrs: Vec<_> = applicants.iter().map(|a| &a.applicant).collect();
    assert!(applicant_addrs.contains(&&applicant1));
    assert!(applicant_addrs.contains(&&applicant2));

    // Accept applicant1
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.update_applicant_status(
        applicant1.clone(),
        1,
        ApplicantStatus::Approved,
    )?;

    // List only "accepted" applicants
    let accepted = arena.arena_team_enrollments.list_applicants(
        1,
        None,
        None,
        Some(ApplicantStatus::Approved),
    )?;
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].applicant, applicant1);

    // List only "pending" applicants
    let pending = arena.arena_team_enrollments.list_applicants(
        1,
        None,
        None,
        Some(ApplicantStatus::Default),
    )?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].applicant, applicant2);

    // Reject applicant 2
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.update_applicant_status(
        applicant2.clone(),
        1,
        ApplicantStatus::Rejected {
            reason: "Not accepted".to_string(),
        },
    )?;

    // List only "rejected" applicants
    let pending = arena.arena_team_enrollments.list_applicants(
        1,
        None,
        None,
        Some(ApplicantStatus::Rejected {
            reason: "".to_string(),
        }),
    )?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].applicant, applicant2);

    // Pagination: start after applicant1
    let paginated = arena.arena_team_enrollments.list_applicants(
        1,
        Some(1),
        Some(applicant2.to_string()),
        None,
    )?;
    assert_eq!(paginated.len(), 1);
    assert_eq!(paginated[0].applicant, applicant1);

    Ok(())
}

#[test]
fn test_applicant_count_tracking() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant1 = mock.addr_make("applicant1");
    let applicant2 = mock.addr_make("applicant2");
    let applicant3 = mock.addr_make("applicant3");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    mock.next_block()?;

    // Initially, no applicants should exist
    let entry = arena.arena_team_enrollments.get_entry(1)?;
    assert_eq!(entry.pending_applicants_count, 0);
    assert_eq!(entry.approved_applicants_count, 0);
    assert_eq!(entry.rejected_applicants_count, 0);

    // Apply with three users (all start as Default/pending status)
    arena.arena_team_enrollments.set_sender(&applicant1);
    arena.arena_team_enrollments.apply(1)?;

    arena.arena_team_enrollments.set_sender(&applicant2);
    arena.arena_team_enrollments.apply(1)?;

    arena.arena_team_enrollments.set_sender(&applicant3);
    arena.arena_team_enrollments.apply(1)?;

    mock.next_block()?;

    // Check counts after applications (all should be pending)
    let entry = arena.arena_team_enrollments.get_entry(1)?;
    assert_eq!(entry.pending_applicants_count, 3);
    assert_eq!(entry.approved_applicants_count, 0);
    assert_eq!(entry.rejected_applicants_count, 0);

    // Approve first applicant (Default -> Approved)
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.update_applicant_status(
        applicant1.to_string(),
        1,
        ApplicantStatus::Approved,
    )?;

    mock.next_block()?;

    // Check counts after approval
    let entry = arena.arena_team_enrollments.get_entry(1)?;
    assert_eq!(entry.pending_applicants_count, 2); // decreased by 1
    assert_eq!(entry.approved_applicants_count, 1); // increased by 1
    assert_eq!(entry.rejected_applicants_count, 0);

    // Reject second applicant (Default -> Rejected)
    arena.arena_team_enrollments.update_applicant_status(
        applicant2.to_string(),
        1,
        ApplicantStatus::Rejected {
            reason: "Not suitable".to_string(),
        },
    )?;

    mock.next_block()?;

    // Check counts after rejection
    let entry = arena.arena_team_enrollments.get_entry(1)?;
    assert_eq!(entry.pending_applicants_count, 1); // decreased by 1
    assert_eq!(entry.approved_applicants_count, 1); // unchanged
    assert_eq!(entry.rejected_applicants_count, 1); // increased by 1

    // Withdraw application from pending applicant
    arena.arena_team_enrollments.set_sender(&applicant3);
    arena.arena_team_enrollments.withdraw_application(1)?;

    mock.next_block()?;

    // Check counts after withdrawal
    let entry = arena.arena_team_enrollments.get_entry(1)?;
    assert_eq!(entry.pending_applicants_count, 0); // decreased by 1
    assert_eq!(entry.approved_applicants_count, 1); // unchanged
    assert_eq!(entry.rejected_applicants_count, 1); // unchanged

    // Final verification: total count should be 2 (withdrew 1)
    let total_count = entry.pending_applicants_count
        + entry.approved_applicants_count
        + entry.rejected_applicants_count;
    assert_eq!(total_count, 2);

    Ok(())
}

#[test]
fn test_list_user_teams() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant = mock.addr_make("applicant");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry, apply, approve, and create team
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    arena.arena_team_enrollments.set_sender(&applicant);
    arena.arena_team_enrollments.apply(1)?;

    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.update_applicant_status(
        applicant.to_string(),
        1,
        ApplicantStatus::Approved,
    )?;

    // Update entry status to Created to instantiate the DAO
    arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Created)?;

    mock.next_block()?;

    // List teams for creator (should have the created team)
    let creator_teams = arena
        .arena_team_enrollments
        .list_teams(creator.to_string(), None, None)?;
    assert_eq!(creator_teams.len(), 1);

    // List teams for approved applicant (should also have the team)
    let applicant_teams =
        arena
            .arena_team_enrollments
            .list_teams(applicant.to_string(), None, None)?;
    assert_eq!(applicant_teams.len(), 1);

    Ok(())
}

#[test]
fn test_entry_status_transitions() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    // Open -> Closed (valid)
    arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Closed)?;

    // Closed -> Open (valid)
    arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Open)?;

    // Open -> Created (valid, terminal)
    arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Created)?;

    mock.next_block()?;

    // Created -> anything (should fail as it's terminal)
    let result = arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Open);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_apply_to_closed_entry() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant = mock.addr_make("applicant");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry and close it
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;
    arena
        .arena_team_enrollments
        .update_entry_status(1, EntryStatus::Closed)?;

    // Try to apply to closed entry (should fail)
    arena.arena_team_enrollments.set_sender(&applicant);
    let result = arena.arena_team_enrollments.apply(1);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_reject_and_withdraw_flow() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, _admin) = setup_arena(&mock)?;

    let creator = mock.addr_make("creator");
    let applicant = mock.addr_make("applicant");

    let dao_config = TeamDaoConfig {
        dao_config: DaoConfig {
            dao_name: "name".to_string(),
            dao_description: "description".to_string(),
            dao_code_id: arena.dao_dao.dao_core.code_id()?,
            cw4_voting_code_id: arena.dao_dao.dao_voting_cw4.code_id()?,
            proposal_single_code_id: arena.dao_dao.dao_proposal_single.code_id()?,
            prepropose_single_code_id: arena.dao_dao.dao_preproprose_single.code_id()?,
            threshold: Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Time(604800),
            image_url: Some("https://example.com/image.png".to_string()),
        },
        cw4_group_code_id: arena.cw4_group.code_id()?,
    };

    // Create entry and apply
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.create_entry(
        dao_config,
        "Description".to_string(),
        "Test Team".to_string(),
        None,
    )?;

    arena.arena_team_enrollments.set_sender(&applicant);
    arena.arena_team_enrollments.apply(1)?;

    // Reject the applicant
    arena.arena_team_enrollments.set_sender(&creator);
    arena.arena_team_enrollments.update_applicant_status(
        applicant.to_string(),
        1,
        ApplicantStatus::Rejected {
            reason: "Not suitable".to_string(),
        },
    )?;

    // Try to withdraw rejected application (should fail)
    arena.arena_team_enrollments.set_sender(&applicant);
    let result = arena.arena_team_enrollments.withdraw_application(1);
    assert!(result.is_err());

    Ok(())
}
