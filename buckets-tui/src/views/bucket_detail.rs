// SPDX-License-Identifier: MIT

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;
use uuid::Uuid;

use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel, bucket_id: Uuid) {
    let bucket = model.buckets.iter().find(|b| b.id == bucket_id);

    let chunks = Layout::vertical([
        Constraint::Length(4), // Bucket info
        Constraint::Min(5),    // Recent commits
        Constraint::Length(8), // Expectations summary
        Constraint::Length(8), // Pebbles summary
    ])
    .split(area);

    // Bucket info section
    if let Some(b) = bucket {
        let info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&b.name, Style::default().fg(Color::White)),
                Span::raw("    "),
                Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.8}", b.id), Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&b.path),
                Span::raw("    "),
                Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
                Span::raw(b.created_at.format("%Y-%m-%d %H:%M").to_string()),
            ]),
        ])
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(info, chunks[0]);
    }

    // Recent commits
    render_section_table(
        frame,
        chunks[1],
        "Recent Commits (Enter to view all)",
        &["Message", "Files", "Date"],
        model
            .detail_commits
            .iter()
            .take(10)
            .map(|c| {
                vec![
                    truncate(&c.message, 50),
                    c.file_count.to_string(),
                    c.created_at.format("%Y-%m-%d %H:%M").to_string(),
                ]
            })
            .collect(),
        if model.selected_index < model.detail_commits.len().min(10) {
            Some(model.selected_index)
        } else {
            None
        },
    );

    // Expectations
    let exp_offset = model.detail_commits.len().min(10);
    render_section_table(
        frame,
        chunks[2],
        "Expectations",
        &["Description", "Status", "Target"],
        model
            .detail_expectations
            .iter()
            .take(5)
            .map(|e| {
                vec![
                    truncate(&e.description, 40),
                    status_colored(&e.status),
                    e.target_bucket.clone().unwrap_or_default(),
                ]
            })
            .collect(),
        if model.selected_index >= exp_offset
            && model.selected_index < exp_offset + model.detail_expectations.len().min(5)
        {
            Some(model.selected_index - exp_offset)
        } else {
            None
        },
    );

    // Pebbles
    let peb_offset = exp_offset + model.detail_expectations.len().min(5);
    render_section_table(
        frame,
        chunks[3],
        "Pebbles",
        &["Description", "Status", "From"],
        model
            .detail_pebbles
            .iter()
            .take(5)
            .map(|p| {
                vec![
                    truncate(&p.description, 40),
                    status_colored(&p.status),
                    p.origin_bucket.clone(),
                ]
            })
            .collect(),
        if model.selected_index >= peb_offset
            && model.selected_index < peb_offset + model.detail_pebbles.len().min(5)
        {
            Some(model.selected_index - peb_offset)
        } else {
            None
        },
    );
}

fn render_section_table(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    headers: &[&str],
    rows_data: Vec<Vec<String>>,
    selected: Option<usize>,
) {
    let header = Row::new(headers.iter().map(|h| h.to_string()).collect::<Vec<_>>()).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row<'_>> = rows_data.into_iter().map(Row::new).collect();

    let n_cols = headers.len();
    let widths: Vec<Constraint> = (0..n_cols)
        .map(|_| Constraint::Percentage((100 / n_cols as u16).max(1)))
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::TOP)
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ratatui::widgets::TableState::default();
    state.select(selected);
    frame.render_stateful_widget(table, area, &mut state);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

fn status_colored(status: &str) -> String {
    // Actual coloring happens via styles in the table; here we just format
    match status {
        "met" | "resolved" => format!("✓ {}", status),
        "pending" | "active" => format!("● {}", status),
        _ => status.to_string(),
    }
}
