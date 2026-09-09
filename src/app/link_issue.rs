//! The link-type picker: choosing how a new link relates the current issue
//! to another one (`L`), before handing off to the Search screen to pick the
//! target (`App::open_search_for_link`, `app::search`'s `SearchPurpose::LinkTo`).
//! Mirrors `transitions.rs`'s open/move/confirm shape.

use crate::domain::{IssueLink, LinkDirection, LinkType, Source};

use super::{async_ops, App, Screen};

/// One selectable row in the link-type picker: a link type paired with one
/// of its two directional labels (e.g. "blocks" or "is blocked by") — a
/// type whose inward/outward labels are identical (e.g. "relates to")
/// contributes only one row, not two indistinguishable ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkPickerRow {
    pub type_name: String,
    pub label: String,
    pub direction: LinkDirection,
}

/// State for the open link-type picker.
#[derive(Clone, Debug, Default)]
pub struct LinkPickerState {
    /// The issue a new link will be created *from*. Set when the picker
    /// opens; `None` otherwise (including in `App::default()`/`new()`).
    pub source_key: Option<String>,
    pub rows: Vec<LinkPickerRow>,
    pub selected: usize,
}

impl App {
    /// Open the link-type picker for the currently viewed/quick-viewed
    /// issue. Doesn't require `list_focus` to be on the quick-view panel,
    /// matching `open_assignee_picker`'s reasoning — opening a modal picker
    /// captures all subsequent input anyway.
    pub fn open_link_picker(&mut self) {
        if self.link_pending {
            self.status = "a link is already in progress".into();
            return;
        }
        let Some(key) = self.link_target_key() else {
            return;
        };
        self.link_picker.source_key = Some(key);
        self.link_picker.rows = self
            .link_types_source()
            .iter()
            .flat_map(|t| {
                let mut rows = vec![LinkPickerRow {
                    type_name: t.name.clone(),
                    label: t.outward.clone(),
                    direction: LinkDirection::Outward,
                }];
                if t.inward != t.outward {
                    rows.push(LinkPickerRow {
                        type_name: t.name.clone(),
                        label: t.inward.clone(),
                        direction: LinkDirection::Inward,
                    });
                }
                rows
            })
            .collect();
        self.link_picker.selected = 0;
        self.link_picker_open = true;
    }

    pub fn close_link_picker(&mut self) {
        self.link_picker_open = false;
    }

    /// The key of the issue a new link should be created *from*: the open
    /// Detail issue, or the quick-view panel's issue if quick view is
    /// showing one. Mirrors `comment_target_key`/`assignee_target_key`
    /// exactly.
    fn link_target_key(&self) -> Option<String> {
        match self.screen {
            Screen::Detail | Screen::Preview | Screen::Edit => {
                self.detail.as_ref().map(|d| d.key.clone())
            }
            _ => self.quick_view_detail().map(|d| d.key.clone()),
        }
    }

    /// The link-type catalog for the current source: the live-fetched list
    /// cached at startup (`App::link_types`, populated by
    /// `dispatch_link_type_discovery`) for a live session, or the baked-in
    /// demo catalog otherwise. Mirrors `assignable_users_source` exactly.
    fn link_types_source(&self) -> Vec<LinkType> {
        if matches!(self.source, Source::Live { .. }) {
            self.link_types.clone()
        } else {
            crate::domain::demo_link_types()
        }
    }

    pub fn link_picker_move(&mut self, delta: isize) {
        let len = self.link_picker.rows.len();
        if len == 0 {
            return;
        }
        let mut idx = self.link_picker.selected as isize + delta;
        if idx < 0 {
            idx = 0;
        }
        if idx >= len as isize {
            idx = len as isize - 1;
        }
        self.link_picker.selected = idx as usize;
    }

    /// Confirm the highlighted link type/direction and hand off to the
    /// Search screen (`App::open_search_for_link`) to pick the target issue.
    pub fn confirm_link_type(&mut self) {
        let Some(source_key) = self.link_picker.source_key.clone() else {
            self.link_picker_open = false;
            return;
        };
        let Some(row) = self
            .link_picker
            .rows
            .get(self.link_picker.selected)
            .cloned()
        else {
            self.link_picker_open = false;
            return;
        };
        self.link_picker_open = false;
        self.open_search_for_link(source_key, row.type_name, row.label, row.direction);
    }

    /// Apply a confirmed link (live if possible, always locally) — called
    /// once a target issue is chosen in the Search screen's `LinkTo` mode.
    /// `target_summary` is `Some` when the chosen row already carried the
    /// target's summary (a local/live-search match); `None` for a direct
    /// "go to issue" entry, in which case the displayed link shows an empty
    /// summary until the issue is next refreshed.
    pub(crate) fn apply_create_issue_link(
        &mut self,
        source_key: String,
        type_name: String,
        label: String,
        direction: LinkDirection,
        target_key: String,
        target_summary: Option<String>,
    ) {
        let link = IssueLink {
            relation: label,
            key: target_key.clone(),
            summary: target_summary.unwrap_or_default(),
        };

        if !matches!(self.source, Source::Live { .. }) {
            self.apply_issue_link_locally(&source_key, link.clone());
            self.status = format!("linked {source_key} — {} {}", link.relation, link.key);
            self.flash(format!("✓ linked to {}", link.key));
            return;
        }

        self.link_generation += 1;
        let generation = self.link_generation;
        self.link_pending = true;
        self.loading = true;
        self.status = format!("↻ linking {source_key} to {target_key}…");
        let tx = self.events_tx.clone();
        async_ops::dispatch_create_issue_link(
            tx, generation, source_key, type_name, direction, target_key, link,
        );
    }

    /// Append `link` to `key`'s links wherever they're cached: the open
    /// Detail and the quick-view detail cache — shared by both the demo/
    /// cache-synchronous path above and `AppEvent::IssueLinkCreated`'s
    /// handler. Mirrors `apply_assignee_locally`'s shape.
    pub(crate) fn apply_issue_link_locally(&mut self, key: &str, link: IssueLink) {
        if let Some(d) = self.detail.as_mut() {
            if d.key == key {
                d.links.push(link.clone());
            }
        }
        if let Some(cached) = self.detail_cache.get_mut(key) {
            cached.links.push(link);
        }
    }
}
