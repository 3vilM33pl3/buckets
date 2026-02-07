// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Row, Table};
use ratatui::Frame;

use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel) {
    // If we have semantic results, show those instead
    if !model.semantic_results.is_empty() {
        render_semantic(frame, area, model);
        return;
    }

    let filter = active_filter(model);
    let rows: Vec<Row<'_>> = model
        .expectations
        .iter()
        .filter(|e| matches_filter(&e.description, filter))
        .map(|e| {
            let status_style = match e.status.as_str() {
                "met" => Style::default().fg(Color::Green),
                "pending" => Style::default().fg(Color::Yellow),
                _ => Style::default(),
            };
            Row::new(vec![
                e.description.clone(),
                e.status.clone(),
                e.source_bucket.clone(),
                e.target_bucket.clone().unwrap_or_default(),
                e.created_at.format("%Y-%m-%d %H:%M").to_string(),
            ])
            .style(status_style)
        })
        .collect();

    let header = Row::new(vec!["Description", "Status", "Source", "Target", "Created"]).style(
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

fn render_semantic(frame: &mut Frame, area: Rect, model: &AppModel) {
    let rows: Vec<Row<'_>> = model
        .semantic_results
        .iter()
        .map(|r| {
            Row::new(vec![
                r.expectation.description.clone(),
                format!("{:.1}%", r.similarity * 100.0),
                r.expectation.status.clone(),
                r.expectation.source_bucket.clone(),
            ])
        })
        .collect();

    let header = Row::new(vec!["Description", "Similarity", "Status", "Source"]).style(
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    );

    let widths = [
        ratatui::layout::Constraint::Percentage(45),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title("Semantic Search Results")
                .borders(Borders::TOP)
                .title_style(Style::default().fg(Color::Magenta)),
        )
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
