// SPDX-License-Identifier: MIT

use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::message::{TabId, ViewId};

/// Bucket summary row for the list view.
#[derive(Debug, Clone)]
pub struct BucketRow {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub commit_count: i64,
    pub expectation_count: i64,
    pub created_at: NaiveDateTime,
}

/// Commit summary row.
#[derive(Debug, Clone)]
pub struct CommitRow {
    pub id: Uuid,
    #[allow(dead_code)]
    pub bucket_name: String,
    pub message: String,
    pub file_count: i64,
    pub created_at: NaiveDateTime,
}

/// File row within a commit.
#[derive(Debug, Clone)]
pub struct FileRow {
    #[allow(dead_code)]
    pub id: Uuid,
    pub file_path: String,
    pub hash: String,
}

/// Expectation row for the expectations list.
#[derive(Debug, Clone)]
pub struct ExpectationRow {
    pub id: Uuid,
    pub description: String,
    pub status: String,
    pub source_bucket: String,
    pub target_bucket: Option<String>,
    pub created_at: NaiveDateTime,
}

/// Pebble row for the pebbles list.
#[derive(Debug, Clone)]
pub struct PebbleRow {
    #[allow(dead_code)]
    pub id: Uuid,
    pub description: String,
    pub origin_bucket: String,
    pub current_bucket: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

/// Semantic search result.
#[derive(Debug, Clone)]
pub struct SemanticResult {
    pub expectation: ExpectationRow,
    pub similarity: f64,
}

/// Graph data for the relationship view.
#[derive(Debug, Clone)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub count: i64,
}

/// Input mode for the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
    Command,
}

/// Full application state.
pub struct AppModel {
    // Navigation
    pub current_tab: TabId,
    pub view_stack: Vec<ViewId>,
    pub input_mode: InputMode,
    pub search_query: String,

    // Data
    pub buckets: Vec<BucketRow>,
    pub commits: Vec<CommitRow>,
    pub files: Vec<FileRow>,
    pub expectations: Vec<ExpectationRow>,
    pub pebbles: Vec<PebbleRow>,
    pub graph_data: Option<GraphData>,
    pub semantic_results: Vec<SemanticResult>,

    // Bucket detail sub-data
    pub detail_commits: Vec<CommitRow>,
    pub detail_expectations: Vec<ExpectationRow>,
    pub detail_pebbles: Vec<PebbleRow>,

    // UI state
    pub selected_index: usize,
    pub show_help: bool,
    pub loading: bool,
    pub db_connected: bool,
    pub last_error: Option<String>,
    pub should_quit: bool,
}

impl AppModel {
    pub fn new() -> Self {
        Self {
            current_tab: TabId::Buckets,
            view_stack: vec![ViewId::BucketsList],
            input_mode: InputMode::Normal,
            search_query: String::new(),

            buckets: Vec::new(),
            commits: Vec::new(),
            files: Vec::new(),
            expectations: Vec::new(),
            pebbles: Vec::new(),
            graph_data: None,
            semantic_results: Vec::new(),

            detail_commits: Vec::new(),
            detail_expectations: Vec::new(),
            detail_pebbles: Vec::new(),

            selected_index: 0,
            show_help: false,
            loading: true,
            db_connected: false,
            last_error: None,
            should_quit: false,
        }
    }

    pub fn current_view(&self) -> &ViewId {
        self.view_stack
            .last()
            .expect("view_stack should never be empty")
    }

    /// Number of items in the currently visible list.
    pub fn current_list_len(&self) -> usize {
        if !self.semantic_results.is_empty()
            && self.input_mode == InputMode::Normal
            && matches!(self.current_view(), ViewId::Expectations)
        {
            return self.semantic_results.len();
        }

        let filter = self.active_filter();
        match self.current_view() {
            ViewId::BucketsList => self.filtered_count(&self.buckets, filter, |b| &b.name),
            ViewId::BucketDetail(_) => {
                self.detail_commits.len()
                    + self.detail_expectations.len()
                    + self.detail_pebbles.len()
            }
            ViewId::CommitHistory(_) => self.filtered_count(&self.commits, filter, |c| &c.message),
            ViewId::CommitDetail(_) => self.filtered_count(&self.files, filter, |f| &f.file_path),
            ViewId::Expectations => {
                self.filtered_count(&self.expectations, filter, |e| &e.description)
            }
            ViewId::Pebbles => self.filtered_count(&self.pebbles, filter, |p| &p.description),
            ViewId::Graph => 0,
        }
    }

    fn active_filter(&self) -> Option<&str> {
        if self.input_mode == InputMode::Filter && !self.search_query.is_empty() {
            Some(&self.search_query)
        } else if self.input_mode == InputMode::Normal && !self.search_query.is_empty() {
            // Keep filter active after submitting
            Some(&self.search_query)
        } else {
            None
        }
    }

    fn filtered_count<T, F>(&self, items: &[T], filter: Option<&str>, field: F) -> usize
    where
        F: Fn(&T) -> &str,
    {
        match filter {
            Some(q) => {
                let q_lower = q.to_lowercase();
                items
                    .iter()
                    .filter(|item| field(item).to_lowercase().contains(&q_lower))
                    .count()
            }
            None => items.len(),
        }
    }

    pub fn clamp_selection(&mut self) {
        let len = self.current_list_len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    pub fn select_next(&mut self) {
        let len = self.current_list_len();
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    pub fn select_last(&mut self) {
        let len = self.current_list_len();
        self.selected_index = if len > 0 { len - 1 } else { 0 };
    }
}
