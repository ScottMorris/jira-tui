//! The link-type picker modal (`L`) — choosing how a new link relates the
//! current issue to another one, before handing off to Search for the
//! target. Mirrors `transition_picker.rs`'s shape.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

use super::{accent, centered_rect_h, maple, muted, selected_style};

pub(crate) fn draw_link_picker(f: &mut Frame, app: &App, area: Rect) {
    let rows = &app.link_picker.rows;
    let height = (rows.len() as u16).saturating_add(4).min(area.height);
    let popup = centered_rect_h(46, height, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent()))
        .title(Span::styled(
            "  link this issue…  ",
            Style::default().fg(accent()).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let selected = i == app.link_picker.selected;
        let cursor = if selected { "▌ " } else { "  " };
        let style = selected_style(
            if selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
            selected,
        );
        lines.push(Line::from(vec![
            Span::styled(
                cursor.to_string(),
                selected_style(Style::default().fg(maple()), selected),
            ),
            Span::styled(row.label.clone(), style),
        ]));
    }
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "no link types available",
            Style::default().fg(muted()),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "⏎ pick · esc/← cancel",
        Style::default().fg(muted()),
    )));
    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}
