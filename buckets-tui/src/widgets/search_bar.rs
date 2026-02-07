// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::model::{AppModel, InputMode};

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel) {
    let (prefix, style) = match model.input_mode {
        InputMode::Filter => (
            "/",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Command => (
            ":",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Normal => {
            if model.search_query.is_empty() {
                return;
            }
            ("/", Style::default().fg(Color::DarkGray))
        }
    };

    let line = Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(&model.search_query, Style::default().fg(Color::White)),
        if model.input_mode != InputMode::Normal {
            Span::styled("█", Style::default().fg(Color::White))
        } else {
            Span::raw("")
        },
    ]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
