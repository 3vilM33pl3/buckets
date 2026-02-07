// SPDX-License-Identifier: MIT

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;
use tokio::sync::mpsc;

use crate::db;
use crate::message::{AppMessage, TabId, ViewId};
use crate::model::{AppModel, InputMode};
use crate::views;
use crate::widgets::{breadcrumb, search_bar, status_bar, tab_bar};

/// The main application, holding state and a channel for async messages.
pub struct App {
    pub model: AppModel,
    pub tx: mpsc::UnboundedSender<AppMessage>,
}

impl App {
    pub fn new(tx: mpsc::UnboundedSender<AppMessage>) -> Self {
        Self {
            model: AppModel::new(),
            tx,
        }
    }

    /// Process a message, updating state and spawning async tasks as needed.
    pub fn update(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::KeyPress(key) => self.handle_key(key),
            AppMessage::NavigateTo(view) => self.navigate_to(view),
            AppMessage::NavigateBack => self.navigate_back(),
            AppMessage::SwitchTab(tab) => self.switch_tab(tab),
            AppMessage::BucketsLoaded(buckets) => {
                self.model.buckets = buckets;
                self.model.loading = false;
                self.model.db_connected = true;
                self.model.last_error = None;
                self.model.clamp_selection();
            }
            AppMessage::CommitsLoaded(_bucket_id, commits) => {
                self.model.commits = commits;
                self.model.loading = false;
                self.model.selected_index = 0;
            }
            AppMessage::FilesLoaded(_commit_id, files) => {
                self.model.files = files;
                self.model.loading = false;
                self.model.selected_index = 0;
            }
            AppMessage::ExpectationsLoaded(expectations) => {
                self.model.expectations = expectations;
                self.model.loading = false;
                self.model.clamp_selection();
            }
            AppMessage::PebblesLoaded(pebbles) => {
                self.model.pebbles = pebbles;
                self.model.loading = false;
                self.model.clamp_selection();
            }
            AppMessage::GraphDataLoaded(data) => {
                self.model.graph_data = Some(data);
                self.model.loading = false;
            }
            AppMessage::SemanticResults(results) => {
                self.model.semantic_results = results;
                self.model.loading = false;
                self.model.selected_index = 0;
            }
            AppMessage::BucketDetailLoaded(_, commits, expectations, pebbles) => {
                self.model.detail_commits = commits;
                self.model.detail_expectations = expectations;
                self.model.detail_pebbles = pebbles;
                self.model.loading = false;
                self.model.selected_index = 0;
            }
            AppMessage::DbError(err) => {
                self.model.last_error = Some(err);
                self.model.loading = false;
                self.model.db_connected = false;
            }
            AppMessage::Tick => {
                self.refresh_current_view();
            }
            AppMessage::Quit => {
                self.model.should_quit = true;
            }
        }
    }

    /// Render the entire UI.
    pub fn view(&self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: tab_bar | breadcrumb | content | search_bar | status_bar
        let show_search =
            self.model.input_mode != InputMode::Normal || !self.model.search_query.is_empty();

        let constraints = if show_search {
            vec![
                Constraint::Length(1), // tab bar
                Constraint::Length(1), // breadcrumb
                Constraint::Min(5),    // content
                Constraint::Length(1), // search bar
                Constraint::Length(1), // status bar
            ]
        } else {
            vec![
                Constraint::Length(1), // tab bar
                Constraint::Length(1), // breadcrumb
                Constraint::Min(5),    // content
                Constraint::Length(1), // status bar
            ]
        };

        let chunks = Layout::vertical(constraints).split(area);

        tab_bar::render(frame, chunks[0], self.model.current_tab);
        breadcrumb::render(frame, chunks[1], &self.model);

        let content_area = chunks[2];
        views::render_view(frame, content_area, &self.model);

        if show_search {
            search_bar::render(frame, chunks[3], &self.model);
            status_bar::render(frame, chunks[4], &self.model);
        } else {
            status_bar::render(frame, chunks[3], &self.model);
        }

        // Help overlay
        if self.model.show_help {
            views::help::render(frame, area);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.model.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Filter => self.handle_filter_key(key),
            InputMode::Command => self.handle_command_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => {
                if self.model.show_help {
                    self.model.show_help = false;
                } else if self.model.view_stack.len() <= 1 {
                    self.model.should_quit = true;
                } else {
                    self.navigate_back();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => self.model.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.model.select_prev(),
            KeyCode::Char('g') | KeyCode::Home => self.model.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.model.select_last(),
            KeyCode::Enter => self.drill_in(),
            KeyCode::Esc => {
                if self.model.show_help {
                    self.model.show_help = false;
                } else if !self.model.search_query.is_empty() {
                    self.model.search_query.clear();
                    self.model.semantic_results.clear();
                    self.model.clamp_selection();
                } else {
                    self.navigate_back();
                }
            }
            KeyCode::Char('/') => {
                self.model.input_mode = InputMode::Filter;
                self.model.search_query.clear();
                self.model.semantic_results.clear();
            }
            KeyCode::Char(':') => {
                self.model.input_mode = InputMode::Command;
                self.model.search_query.clear();
            }
            KeyCode::Char('?') => {
                self.model.show_help = !self.model.show_help;
            }
            KeyCode::Char('r') => {
                self.model.loading = true;
                self.refresh_current_view();
            }
            KeyCode::Tab => {
                let next = (self.model.current_tab.index() + 1) % TabId::all().len();
                if let Some(tab) = TabId::from_index(next) {
                    self.switch_tab(tab);
                }
            }
            KeyCode::Char('1') => self.switch_tab(TabId::Buckets),
            KeyCode::Char('2') => self.switch_tab(TabId::Expectations),
            KeyCode::Char('3') => self.switch_tab(TabId::Pebbles),
            KeyCode::Char('4') => self.switch_tab(TabId::Graph),
            _ => {}
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.model.input_mode = InputMode::Normal;
                self.model.search_query.clear();
                self.model.clamp_selection();
            }
            KeyCode::Enter => {
                self.model.input_mode = InputMode::Normal;
                self.model.selected_index = 0;
            }
            KeyCode::Backspace => {
                self.model.search_query.pop();
                self.model.selected_index = 0;
            }
            KeyCode::Char(c) => {
                self.model.search_query.push(c);
                self.model.selected_index = 0;
            }
            _ => {}
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.model.input_mode = InputMode::Normal;
                self.model.search_query.clear();
            }
            KeyCode::Enter => {
                self.model.input_mode = InputMode::Normal;
                let query = self.model.search_query.clone();
                if let Some(search_term) = query.strip_prefix("semantic ") {
                    let term = search_term.to_string();
                    self.model.loading = true;
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        match db::semantic_search(&term).await {
                            Ok(results) => {
                                let _ = tx.send(AppMessage::SemanticResults(results));
                            }
                            Err(e) => {
                                let _ = tx.send(AppMessage::DbError(e.to_string()));
                            }
                        }
                    });
                }
                self.model.search_query.clear();
            }
            KeyCode::Backspace => {
                self.model.search_query.pop();
            }
            KeyCode::Char(c) => {
                self.model.search_query.push(c);
            }
            _ => {}
        }
    }

    fn navigate_to(&mut self, view: ViewId) {
        self.model.selected_index = 0;
        self.model.search_query.clear();
        self.model.semantic_results.clear();
        self.model.loading = true;
        self.model.view_stack.push(view.clone());
        self.load_data_for_view(&view);
    }

    fn navigate_back(&mut self) {
        if self.model.view_stack.len() > 1 {
            self.model.view_stack.pop();
            self.model.selected_index = 0;
            self.model.search_query.clear();
            self.model.semantic_results.clear();
            self.model.loading = true;
            let current = self.model.current_view().clone();
            self.load_data_for_view(&current);
        }
    }

    fn switch_tab(&mut self, tab: TabId) {
        self.model.current_tab = tab;
        self.model.selected_index = 0;
        self.model.search_query.clear();
        self.model.semantic_results.clear();
        self.model.loading = true;

        let view = match tab {
            TabId::Buckets => ViewId::BucketsList,
            TabId::Expectations => ViewId::Expectations,
            TabId::Pebbles => ViewId::Pebbles,
            TabId::Graph => ViewId::Graph,
        };

        self.model.view_stack = vec![view.clone()];
        self.load_data_for_view(&view);
    }

    fn drill_in(&mut self) {
        let idx = self.model.selected_index;
        let filter = if !self.model.search_query.is_empty() {
            Some(self.model.search_query.clone())
        } else {
            None
        };

        match self.model.current_view().clone() {
            ViewId::BucketsList => {
                let filtered: Vec<_> = self
                    .model
                    .buckets
                    .iter()
                    .filter(|b| match &filter {
                        Some(q) => b.name.to_lowercase().contains(&q.to_lowercase()),
                        None => true,
                    })
                    .collect();
                if let Some(bucket) = filtered.get(idx) {
                    self.navigate_to(ViewId::BucketDetail(bucket.id));
                }
            }
            ViewId::BucketDetail(bucket_id) => {
                // Drill into commit history
                let commit_count = self.model.detail_commits.len().min(10);
                if idx < commit_count {
                    self.navigate_to(ViewId::CommitHistory(bucket_id));
                }
            }
            ViewId::CommitHistory(_) => {
                let filtered: Vec<_> = self
                    .model
                    .commits
                    .iter()
                    .filter(|c| match &filter {
                        Some(q) => c.message.to_lowercase().contains(&q.to_lowercase()),
                        None => true,
                    })
                    .collect();
                if let Some(commit) = filtered.get(idx) {
                    self.navigate_to(ViewId::CommitDetail(commit.id));
                }
            }
            ViewId::Expectations => {
                // Navigate to source bucket detail
                if !self.model.semantic_results.is_empty() {
                    // From semantic results
                    if let Some(result) = self.model.semantic_results.get(idx) {
                        let exp_id = result.expectation.id;
                        if let Some(exp) = self.model.expectations.iter().find(|e| e.id == exp_id) {
                            if let Some(bucket) = self
                                .model
                                .buckets
                                .iter()
                                .find(|b| b.name == exp.source_bucket)
                            {
                                self.navigate_to(ViewId::BucketDetail(bucket.id));
                            }
                        }
                    }
                } else {
                    let filtered: Vec<_> = self
                        .model
                        .expectations
                        .iter()
                        .filter(|e| match &filter {
                            Some(q) => e.description.to_lowercase().contains(&q.to_lowercase()),
                            None => true,
                        })
                        .collect();
                    if let Some(exp) = filtered.get(idx) {
                        if let Some(bucket) = self
                            .model
                            .buckets
                            .iter()
                            .find(|b| b.name == exp.source_bucket)
                        {
                            self.navigate_to(ViewId::BucketDetail(bucket.id));
                        }
                    }
                }
            }
            ViewId::Pebbles => {
                let filtered: Vec<_> = self
                    .model
                    .pebbles
                    .iter()
                    .filter(|p| match &filter {
                        Some(q) => p.description.to_lowercase().contains(&q.to_lowercase()),
                        None => true,
                    })
                    .collect();
                if let Some(pebble) = filtered.get(idx) {
                    if let Some(bucket) = self
                        .model
                        .buckets
                        .iter()
                        .find(|b| b.name == pebble.current_bucket)
                    {
                        self.navigate_to(ViewId::BucketDetail(bucket.id));
                    }
                }
            }
            _ => {}
        }
    }

    fn load_data_for_view(&self, view: &ViewId) {
        let tx = self.tx.clone();
        match view {
            ViewId::BucketsList => {
                tokio::spawn(async move {
                    match db::fetch_buckets().await {
                        Ok(buckets) => {
                            let _ = tx.send(AppMessage::BucketsLoaded(buckets));
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::DbError(e.to_string()));
                        }
                    }
                });
            }
            ViewId::BucketDetail(id) => {
                let id = *id;
                tokio::spawn(async move {
                    let commits = db::fetch_commits(id).await.unwrap_or_default();
                    let expectations = db::fetch_expectations_for_bucket(id)
                        .await
                        .unwrap_or_default();
                    let pebbles = db::fetch_pebbles_for_bucket(id).await.unwrap_or_default();
                    let _ = tx.send(AppMessage::BucketDetailLoaded(
                        id,
                        commits,
                        expectations,
                        pebbles,
                    ));
                });
            }
            ViewId::CommitHistory(bucket_id) => {
                let id = *bucket_id;
                tokio::spawn(async move {
                    match db::fetch_commits(id).await {
                        Ok(commits) => {
                            let _ = tx.send(AppMessage::CommitsLoaded(id, commits));
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::DbError(e.to_string()));
                        }
                    }
                });
            }
            ViewId::CommitDetail(commit_id) => {
                let id = *commit_id;
                tokio::spawn(async move {
                    match db::fetch_files(id).await {
                        Ok(files) => {
                            let _ = tx.send(AppMessage::FilesLoaded(id, files));
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::DbError(e.to_string()));
                        }
                    }
                });
            }
            ViewId::Expectations => {
                tokio::spawn(async move {
                    match db::fetch_expectations().await {
                        Ok(expectations) => {
                            let _ = tx.send(AppMessage::ExpectationsLoaded(expectations));
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::DbError(e.to_string()));
                        }
                    }
                });
            }
            ViewId::Pebbles => {
                tokio::spawn(async move {
                    match db::fetch_pebbles().await {
                        Ok(pebbles) => {
                            let _ = tx.send(AppMessage::PebblesLoaded(pebbles));
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::DbError(e.to_string()));
                        }
                    }
                });
            }
            ViewId::Graph => {
                tokio::spawn(async move {
                    match db::fetch_graph_data().await {
                        Ok(data) => {
                            let _ = tx.send(AppMessage::GraphDataLoaded(data));
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::DbError(e.to_string()));
                        }
                    }
                });
            }
        }
    }

    pub fn refresh_current_view(&self) {
        let view = self.model.current_view().clone();
        self.load_data_for_view(&view);

        // Also refresh buckets list if not currently on it (for drill-down name resolution)
        if !matches!(view, ViewId::BucketsList) {
            let tx = self.tx.clone();
            tokio::spawn(async move {
                if let Ok(buckets) = db::fetch_buckets().await {
                    let _ = tx.send(AppMessage::BucketsLoaded(buckets));
                }
            });
        }
    }

    /// Initial data load.
    pub fn load_initial_data(&self) {
        self.load_data_for_view(&ViewId::BucketsList);
    }
}
