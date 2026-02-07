// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel) {
    let db_indicator = if model.db_connected {
        Span::styled("● connected", Style::default().fg(Color::Green))
    } else {
        Span::styled("● disconnected", Style::default().fg(Color::Red))
    };

    let mut spans = vec![
        Span::styled("db: ", Style::default().fg(Color::DarkGray)),
        db_indicator,
        Span::raw("  "),
    ];

    if model.loading {
        spans.push(Span::styled(
            "⟳ loading",
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::raw("  "));
    }

    if let Some(err) = &model.last_error {
        let truncated = if err.len() > 40 {
            format!("{}...", &err[..37])
        } else {
            err.clone()
        };
        spans.push(Span::styled(truncated, Style::default().fg(Color::Red)));
        spans.push(Span::raw("  "));
    }

    spans.push(Span::styled(
        "r:refresh  /:filter  ?:help  q:quit",
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
