use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let stats = &app.stats;
    let line = Line::from(vec![
        Span::styled(" Requests: ", Style::default().fg(Color::Cyan)),
        Span::raw(stats.total_requests.to_string()),
        Span::raw("  "),
        Span::styled("Responses: ", Style::default().fg(Color::Cyan)),
        Span::raw(stats.total_responses.to_string()),
        Span::raw("  "),
        Span::styled("Connections: ", Style::default().fg(Color::Cyan)),
        Span::raw(stats.active_connections.to_string()),
        Span::raw("  "),
        Span::styled("Captured: ", Style::default().fg(Color::Cyan)),
        Span::raw(format_bytes(stats.bytes_captured)),
    ]);

    let paragraph = Paragraph::new(line).block(
        Block::default()
            .title(" Status ")
            .borders(Borders::ALL),
    );
    f.render_widget(paragraph, area);
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_units() {
        assert_eq!(format_bytes(100), "100B");
        assert_eq!(format_bytes(1500), "1.5KB");
        assert_eq!(format_bytes(1_500_000), "1.4MB");
    }
}
