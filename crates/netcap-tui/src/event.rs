use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::app::App;

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true
        }
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Tab => app.toggle_tab(),
        KeyCode::Enter => {
            if app.selected_exchange().is_some() {
                app.tab = crate::app::AppTab::Detail;
            }
        }
        _ => {}
    }
}

pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn quit_on_q() {
        let mut app = App::new();
        handle_key_event(&mut app, key_event(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn quit_on_esc() {
        let mut app = App::new();
        handle_key_event(&mut app, key_event(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn quit_on_ctrl_c() {
        let mut app = App::new();
        handle_key_event(&mut app, ctrl_key(KeyCode::Char('c')));
        assert!(app.should_quit);
    }

    #[test]
    fn j_k_navigation() {
        let mut app = App::new();
        // Add exchanges for navigation
        use bytes::Bytes;
        use chrono::Utc;
        use http::{HeaderMap, Method, Version};
        use netcap_core::capture::exchange::{CapturedExchange, CapturedRequest};
        use uuid::Uuid;

        for _ in 0..3 {
            app.add_exchange(CapturedExchange {
                request: CapturedRequest {
                    id: Uuid::now_v7(),
                    session_id: Uuid::now_v7(),
                    connection_id: Uuid::now_v7(),
                    sequence_number: 0,
                    timestamp: Utc::now(),
                    method: Method::GET,
                    uri: "http://test.com/".parse().unwrap(),
                    version: Version::HTTP_11,
                    headers: HeaderMap::new(),
                    body: Bytes::new(),
                    body_truncated: false,
                    tls_info: None,
                },
                response: None,
            });
        }

        handle_key_event(&mut app, key_event(KeyCode::Char('j')));
        assert_eq!(app.selected_index, 1);
        handle_key_event(&mut app, key_event(KeyCode::Char('k')));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn tab_toggles() {
        let mut app = App::new();
        assert_eq!(app.tab, crate::app::AppTab::RequestList);
        handle_key_event(&mut app, key_event(KeyCode::Tab));
        assert_eq!(app.tab, crate::app::AppTab::Detail);
    }
}
