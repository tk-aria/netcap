use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .exchanges
        .iter()
        .map(|ex| {
            let method = ex.request.method.to_string();
            let host = ex.request.uri.host().unwrap_or("-");
            let path = ex.request.uri.path();
            let status = ex
                .response
                .as_ref()
                .map(|r| r.status.as_u16().to_string())
                .unwrap_or_else(|| "---".to_string());
            let color = match ex.response.as_ref().map(|r| r.status.as_u16()) {
                Some(200..=299) => Color::Green,
                Some(300..=399) => Color::Yellow,
                Some(400..=499) => Color::Red,
                Some(500..=599) => Color::Magenta,
                _ => Color::Gray,
            };
            ListItem::new(Line::from(format!(
                "{:<7} {}{} → {}",
                method, host, path, status
            )))
            .style(Style::default().fg(color))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Requests (↑↓/jk: navigate, Enter: detail, Tab: switch, q: quit) ")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(list, area, &mut state);
}
