// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Row, Table};
use ratatui::Frame;
use uuid::Uuid;

use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel, _bucket_id: Uuid) {
    let filter = active_filter(model);
    let rows: Vec<Row<'_>> = model
        .commits
        .iter()
        .filter(|c| matches_filter(&c.message, filter))
        .map(|c| {
            Row::new(vec![
                format!("{:.8}", c.id),
                c.message.clone(),
                c.file_count.to_string(),
                c.created_at.format("%Y-%m-%d %H:%M").to_string(),
            ])
        })
        .collect();

    let header = Row::new(vec!["ID", "Message", "Files", "Date"]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let widths = [
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Percentage(50),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(model.selected_index));
    frame.render_stateful_widget(table, area, &mut state);
}

fn active_filter(model: &AppModel) -> Option<&str> {
    if !model.search_query.is_empty() {
        Some(&model.search_query)
    } else {
        None
    }
}

fn matches_filter(text: &str, filter: Option<&str>) -> bool {
    match filter {
        Some(q) => text.to_lowercase().contains(&q.to_lowercase()),
        None => true,
    }
}
