// SPDX-License-Identifier: MIT

use crossterm::event::KeyEvent;
use uuid::Uuid;

use crate::model::{
    BucketRow, CommitRow, ExpectationRow, FileRow, GraphData, PebbleRow, SemanticResult,
};

/// Top-level tabs in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Buckets,
    Expectations,
    Pebbles,
    Graph,
}

impl TabId {
    pub fn all() -> &'static [TabId] {
        &[
            TabId::Buckets,
            TabId::Expectations,
            TabId::Pebbles,
            TabId::Graph,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            TabId::Buckets => "Buckets",
            TabId::Expectations => "Expectations",
            TabId::Pebbles => "Pebbles",
            TabId::Graph => "Graph",
        }
    }

    pub fn index(self) -> usize {
        match self {
            TabId::Buckets => 0,
            TabId::Expectations => 1,
            TabId::Pebbles => 2,
            TabId::Graph => 3,
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(TabId::Buckets),
            1 => Some(TabId::Expectations),
            2 => Some(TabId::Pebbles),
            3 => Some(TabId::Graph),
            _ => None,
        }
    }
}

/// Identifies which view is currently displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewId {
    BucketsList,
    BucketDetail(Uuid),
    CommitHistory(Uuid),
    CommitDetail(Uuid),
    Expectations,
    Pebbles,
    Graph,
}

/// All messages the app can handle (TEA pattern).
#[derive(Debug)]
#[allow(dead_code)]
pub enum AppMessage {
    // Navigation
    NavigateTo(ViewId),
    NavigateBack,
    SwitchTab(TabId),

    // Input
    KeyPress(KeyEvent),

    // Data responses
    BucketsLoaded(Vec<BucketRow>),
    CommitsLoaded(Uuid, Vec<CommitRow>),
    FilesLoaded(Uuid, Vec<FileRow>),
    ExpectationsLoaded(Vec<ExpectationRow>),
    PebblesLoaded(Vec<PebbleRow>),
    GraphDataLoaded(GraphData),
    SemanticResults(Vec<SemanticResult>),
    BucketDetailLoaded(Uuid, Vec<CommitRow>, Vec<ExpectationRow>, Vec<PebbleRow>),

    // System
    DbError(String),
    Tick,
    Quit,
}
