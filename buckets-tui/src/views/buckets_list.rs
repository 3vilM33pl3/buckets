// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Row, Table};
use ratatui::Frame;

use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel) {
    let filter = active_filter(model);
    let rows: Vec<Row<'_>> = model
        .buckets
        .iter()
        .filter(|b| matches_filter(&b.name, filter))
        .map(|b| {
            Row::new(vec![
                b.name.clone(),
                b.path.clone(),
                b.commit_count.to_string(),
                b.expectation_count.to_string(),
                b.created_at.format("%Y-%m-%d %H:%M").to_string(),
            ])
        })
        .collect();

    let selected = model.selected_index;
    let header = Row::new(vec!["Name", "Path", "Commits", "Expectations", "Created"])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let widths = [
        ratatui::layout::Constraint::Percentage(20),
        ratatui::layout::Constraint::Percentage(30),
        ratatui::layout::Constraint::Percentage(12),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(23),
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
    state.select(Some(selected));
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
