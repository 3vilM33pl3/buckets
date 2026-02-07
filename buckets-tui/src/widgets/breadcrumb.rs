// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::message::ViewId;
use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel) {
    let mut crumbs: Vec<Span<'_>> = Vec::new();

    for (i, view) in model.view_stack.iter().enumerate() {
        if i > 0 {
            crumbs.push(Span::styled(" > ", Style::default().fg(Color::DarkGray)));
        }

        let label = match view {
            ViewId::BucketsList => "Buckets".to_string(),
            ViewId::BucketDetail(id) => model
                .buckets
                .iter()
                .find(|b| b.id == *id)
                .map(|b| b.name.clone())
                .unwrap_or_else(|| format!("{:.8}", id)),
            ViewId::CommitHistory(_) => "Commits".to_string(),
            ViewId::CommitDetail(id) => format!("Commit {:.8}", id),
            ViewId::Expectations => "Expectations".to_string(),
            ViewId::Pebbles => "Pebbles".to_string(),
            ViewId::Graph => "Graph".to_string(),
        };

        let style = if i == model.view_stack.len() - 1 {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        crumbs.push(Span::styled(label, style));
    }

    let line = Line::from(crumbs);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
