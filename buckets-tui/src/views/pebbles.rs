// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Row, Table};
use ratatui::Frame;

use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel) {
    let filter = active_filter(model);
    let rows: Vec<Row<'_>> = model
        .pebbles
        .iter()
        .filter(|p| matches_filter(&p.description, filter))
        .map(|p| {
            let status_style = match p.status.as_str() {
                "resolved" => Style::default().fg(Color::Green),
                "active" => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            };
            Row::new(vec![
                p.description.clone(),
                p.status.clone(),
                p.origin_bucket.clone(),
                p.current_bucket.clone(),
                p.created_at.format("%Y-%m-%d %H:%M").to_string(),
            ])
            .style(status_style)
        })
        .collect();

    let header = Row::new(vec![
        "Description",
        "Status",
        "Origin",
        "Current",
        "Created",
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let widths = [
        ratatui::layout::Constraint::Percentage(35),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(18),
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
