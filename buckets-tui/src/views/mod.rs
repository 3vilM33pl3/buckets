// SPDX-License-Identifier: MIT

pub mod bucket_detail;
pub mod buckets_list;
pub mod commit_detail;
pub mod commit_history;
pub mod expectations;
pub mod graph;
pub mod help;
pub mod pebbles;

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::message::ViewId;
use crate::model::AppModel;

/// Dispatch rendering to the appropriate view based on current state.
pub fn render_view(frame: &mut Frame, area: Rect, model: &AppModel) {
    match model.current_view() {
        ViewId::BucketsList => buckets_list::render(frame, area, model),
        ViewId::BucketDetail(id) => bucket_detail::render(frame, area, model, *id),
        ViewId::CommitHistory(id) => commit_history::render(frame, area, model, *id),
        ViewId::CommitDetail(id) => commit_detail::render(frame, area, model, *id),
        ViewId::Expectations => expectations::render(frame, area, model),
        ViewId::Pebbles => pebbles::render(frame, area, model),
        ViewId::Graph => graph::render(frame, area, model),
    }
}
