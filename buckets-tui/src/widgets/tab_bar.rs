// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Tabs;
use ratatui::Frame;

use crate::message::TabId;

pub fn render(frame: &mut Frame, area: Rect, current_tab: TabId) {
    let titles: Vec<Line<'_>> = TabId::all()
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let num = format!("{}:", i + 1);
            Line::from(vec![
                Span::styled(num, Style::default().fg(Color::DarkGray)),
                Span::raw(tab.label()),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .select(current_tab.index())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" | "));

    frame.render_widget(tabs, area);
}
