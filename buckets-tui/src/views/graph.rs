// SPDX-License-Identifier: MIT

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::model::AppModel;

pub fn render(frame: &mut Frame, area: Rect, model: &AppModel) {
    let graph = match &model.graph_data {
        Some(g) => g,
        None => {
            let placeholder = Paragraph::new("Loading graph data...")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::NONE));
            frame.render_widget(placeholder, area);
            return;
        }
    };

    if graph.nodes.is_empty() {
        let placeholder = Paragraph::new("No buckets found.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(placeholder, area);
        return;
    }

    // Build ASCII graph lines
    let mut lines: Vec<Line<'_>> = Vec::new();

    lines.push(Line::from(Span::styled(
        "Bucket Relationships (via Expectations)",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // List each node and its outgoing edges
    for node in &graph.nodes {
        let outgoing: Vec<_> = graph.edges.iter().filter(|e| e.from == node.id).collect();
        let incoming: Vec<_> = graph.edges.iter().filter(|e| e.to == node.id).collect();

        let mut spans: Vec<Span<'_>> = vec![Span::styled(
            format!("[{}]", node.name),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )];

        if outgoing.is_empty() && incoming.is_empty() {
            spans.push(Span::styled(
                "  (no connections)",
                Style::default().fg(Color::DarkGray),
            ));
        }

        lines.push(Line::from(spans));

        for edge in &outgoing {
            let target_name = graph
                .nodes
                .iter()
                .find(|n| n.id == edge.to)
                .map(|n| n.name.as_str())
                .unwrap_or("?");

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("──expects(", Style::default().fg(Color::DarkGray)),
                Span::styled(edge.count.to_string(), Style::default().fg(Color::Yellow)),
                Span::styled(")──▸ ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("[{}]", target_name),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

        for edge in &incoming {
            let source_name = graph
                .nodes
                .iter()
                .find(|n| n.id == edge.from)
                .map(|n| n.name.as_str())
                .unwrap_or("?");

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("◂──expects(", Style::default().fg(Color::DarkGray)),
                Span::styled(edge.count.to_string(), Style::default().fg(Color::Yellow)),
                Span::styled(")── ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("[{}]", source_name),
                    Style::default().fg(Color::Blue),
                ),
            ]));
        }

        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    frame.render_widget(paragraph, area);
}
