// SPDX-License-Identifier: MIT

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect) {
    // Center a popup
    let popup_area = centered_rect(60, 70, area);

    let lines = vec![
        Line::from(Span::styled(
            " Keybindings ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        section("Navigation"),
        key_line("j / ↓", "Select next item"),
        key_line("k / ↑", "Select previous item"),
        key_line("Enter", "Drill into selected"),
        key_line("Esc", "Go back / cancel search"),
        key_line("q", "Quit (at root view)"),
        key_line("g / Home", "Jump to first item"),
        key_line("G / End", "Jump to last item"),
        Line::from(""),
        section("Tabs"),
        key_line("Tab", "Next tab"),
        key_line("1-4", "Jump to tab"),
        Line::from(""),
        section("Search"),
        key_line("/", "Start text filter"),
        key_line(":semantic ...", "Semantic search (expectations)"),
        Line::from(""),
        section("Other"),
        key_line("r", "Refresh data"),
        key_line("?", "Toggle this help"),
    ];

    frame.render_widget(Clear, popup_area);
    let popup = Paragraph::new(lines).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(popup, popup_area);
}

fn section(title: &str) -> Line<'_> {
    Line::from(Span::styled(
        format!(" {}", title),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<16}", key),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(desc),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [y] = vertical.areas(r);
    let [x] = horizontal.areas(y);
    x
}
