use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(ex) = app.selected_exchange() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Method: ", Style::default().fg(Color::Cyan)),
                Span::raw(ex.request.method.to_string()),
            ]),
            Line::from(vec![
                Span::styled("URI:    ", Style::default().fg(Color::Cyan)),
                Span::raw(ex.request.uri.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Version:", Style::default().fg(Color::Cyan)),
                Span::raw(format!(" {:?}", ex.request.version)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Request Headers:",
                Style::default().fg(Color::Yellow),
            )),
        ];
        for (key, value) in ex.request.headers.iter() {
            lines.push(Line::from(format!(
                "  {}: {}",
                key,
                value.to_str().unwrap_or("<binary>")
            )));
        }

        if let Some(ref resp) = ex.response {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::raw(resp.status.to_string()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Latency:", Style::default().fg(Color::Cyan)),
                Span::raw(format!(" {:.1}ms", resp.latency.as_secs_f64() * 1000.0)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Response Headers:",
                Style::default().fg(Color::Yellow),
            )));
            for (key, value) in resp.headers.iter() {
                lines.push(Line::from(format!(
                    "  {}: {}",
                    key,
                    value.to_str().unwrap_or("<binary>")
                )));
            }
        }

        lines
    } else {
        vec![Line::from("No request selected")]
    };

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .title(" Detail (Tab: back to list) ")
            .borders(Borders::ALL),
    );
    f.render_widget(paragraph, area);
}
