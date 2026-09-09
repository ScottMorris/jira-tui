//! Link-type picker and issue-link creation tests.

use super::super::*;
use super::support::*;

#[test]
fn open_link_picker_builds_one_row_per_direction_skipping_duplicate_labels() {
    let mut app = demo_app();
    app.open_detail();
    app.open_link_picker();
    assert!(app.link_picker_open);
    let labels: Vec<&str> = app
        .link_picker
        .rows
        .iter()
        .map(|r| r.label.as_str())
        .collect();
    // Demo catalog: Blocks, Duplicate, Relates, Cloners — "Relates" has
    // identical inward/outward labels, so it contributes only one row.
    assert_eq!(
        labels,
        vec![
            "blocks",
            "is blocked by",
            "duplicates",
            "is duplicated by",
            "relates to",
            "clones",
            "is cloned by",
        ]
    );
}

#[test]
fn link_picker_move_clamps_to_bounds() {
    let mut app = demo_app();
    app.open_detail();
    app.open_link_picker();
    let len = app.link_picker.rows.len();
    app.link_picker_move(-5);
    assert_eq!(app.link_picker.selected, 0);
    app.link_picker_move(1000);
    assert_eq!(app.link_picker.selected, len - 1);
}

#[test]
fn confirm_link_type_opens_search_in_link_to_mode() {
    let mut app = demo_app();
    app.selected = 0;
    app.open_detail();
    let key = app.detail.as_ref().unwrap().key.clone();
    app.open_link_picker();
    app.link_picker.selected = 0; // "blocks"
    app.confirm_link_type();

    assert!(!app.link_picker_open);
    assert_eq!(app.screen, Screen::Search);
    match &app.search.purpose {
        SearchPurpose::LinkTo {
            source_key,
            type_name,
            label,
            direction,
        } => {
            assert_eq!(source_key, &key);
            assert_eq!(type_name, "Blocks");
            assert_eq!(label, "blocks");
            assert_eq!(*direction, crate::domain::LinkDirection::Outward);
        }
        other => panic!("expected LinkTo, got {other:?}"),
    }
}

#[test]
fn apply_create_issue_link_in_demo_mode_pushes_the_link_locally() {
    let mut app = demo_app();
    app.selected = 0;
    app.open_detail();
    let key = app.detail.as_ref().unwrap().key.clone();
    let before = app.detail.as_ref().unwrap().links.len();

    app.apply_create_issue_link(
        key,
        "Blocks".into(),
        "blocks".into(),
        crate::domain::LinkDirection::Outward,
        "DS-9999".into(),
        Some("Some other issue".into()),
    );

    let links = &app.detail.as_ref().unwrap().links;
    assert_eq!(links.len(), before + 1);
    let new_link = links.last().unwrap();
    assert_eq!(new_link.relation, "blocks");
    assert_eq!(new_link.key, "DS-9999");
    assert_eq!(new_link.summary, "Some other issue");
}

#[tokio::test]
async fn apply_create_issue_link_against_a_live_source_dispatches_and_applies_on_completion() {
    let _guard = crate::test_support::lock_env_async().await;
    let mut app = live_app();
    let key = app.issues[0].key.clone();
    app.detail = Some(crate::domain::demo_detail(&key));
    app.screen = Screen::Detail;

    app.apply_create_issue_link(
        key,
        "Blocks".into(),
        "blocks".into(),
        crate::domain::LinkDirection::Outward,
        "DS-9999".into(),
        None,
    );
    assert!(app.loading);
    assert!(app.link_pending);

    let event = next_event(&mut app).await;
    app.apply_event(event);
    assert!(!app.loading);
    assert!(!app.link_pending);
    assert!(app
        .detail
        .as_ref()
        .unwrap()
        .links
        .iter()
        .any(|l| l.key == "DS-9999" && l.relation == "blocks"));
}

/// Mirrors `open_transitions_refuses_to_reopen_while_one_is_in_flight`.
#[tokio::test]
async fn open_link_picker_refuses_to_reopen_while_a_link_is_in_flight() {
    let _guard = crate::test_support::lock_env_async().await;
    let mut app = live_app();
    let key = app.issues[0].key.clone();
    app.detail = Some(crate::domain::demo_detail(&key));
    app.screen = Screen::Detail;

    app.apply_create_issue_link(
        key,
        "Blocks".into(),
        "blocks".into(),
        crate::domain::LinkDirection::Outward,
        "DS-9999".into(),
        None,
    );
    assert!(app.link_pending);
    let generation = app.link_generation;

    app.open_link_picker();
    assert!(
        !app.link_picker_open,
        "the picker must not reopen while a link is in flight"
    );

    let event = next_event(&mut app).await;
    app.apply_event(event);
    assert!(!app.link_pending);
    assert_eq!(app.link_generation, generation);
}
