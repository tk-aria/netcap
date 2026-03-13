use netcap_core::capture::exchange::CapturedExchange;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppTab {
    RequestList,
    Detail,
}

pub struct CaptureStats {
    pub total_requests: u64,
    pub total_responses: u64,
    pub active_connections: u32,
    pub bytes_captured: u64,
}

pub struct App {
    pub tab: AppTab,
    pub exchanges: Vec<CapturedExchange>,
    pub selected_index: usize,
    pub should_quit: bool,
    pub stats: CaptureStats,
}

impl App {
    pub fn new() -> Self {
        Self {
            tab: AppTab::RequestList,
            exchanges: Vec::new(),
            selected_index: 0,
            should_quit: false,
            stats: CaptureStats {
                total_requests: 0,
                total_responses: 0,
                active_connections: 0,
                bytes_captured: 0,
            },
        }
    }

    pub fn next(&mut self) {
        if self.selected_index < self.exchanges.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn previous(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn add_exchange(&mut self, exchange: CapturedExchange) {
        self.stats.total_requests += 1;
        self.stats.bytes_captured += exchange.request.body.len() as u64;
        if let Some(ref resp) = exchange.response {
            self.stats.total_responses += 1;
            self.stats.bytes_captured += resp.body.len() as u64;
        }
        self.exchanges.push(exchange);
    }

    pub fn selected_exchange(&self) -> Option<&CapturedExchange> {
        self.exchanges.get(self.selected_index)
    }

    pub fn toggle_tab(&mut self) {
        self.tab = match self.tab {
            AppTab::RequestList => AppTab::Detail,
            AppTab::Detail => AppTab::RequestList,
        };
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, StatusCode, Version};
    use netcap_core::capture::exchange::{CapturedRequest, CapturedResponse};
    use uuid::Uuid;

    fn make_exchange() -> CapturedExchange {
        CapturedExchange {
            request: CapturedRequest {
                id: Uuid::now_v7(),
                session_id: Uuid::now_v7(),
                connection_id: Uuid::now_v7(),
                sequence_number: 0,
                timestamp: Utc::now(),
                method: Method::GET,
                uri: "http://example.com/".parse().unwrap(),
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::from("abc"),
                body_truncated: false,
                tls_info: None,
            },
            response: Some(CapturedResponse {
                id: Uuid::now_v7(),
                request_id: Uuid::now_v7(),
                timestamp: Utc::now(),
                status: StatusCode::OK,
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::from("defgh"),
                body_truncated: false,
                latency: std::time::Duration::from_millis(10),
                ttfb: std::time::Duration::from_millis(5),
            }),
        }
    }

    #[test]
    fn new_app_defaults() {
        let app = App::new();
        assert_eq!(app.tab, AppTab::RequestList);
        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(app.exchanges.is_empty());
    }

    #[test]
    fn next_moves_index() {
        let mut app = App::new();
        app.add_exchange(make_exchange());
        app.add_exchange(make_exchange());
        app.add_exchange(make_exchange());
        assert_eq!(app.selected_index, 0);
        app.next();
        assert_eq!(app.selected_index, 1);
        app.next();
        assert_eq!(app.selected_index, 2);
        // Should not go past end
        app.next();
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn previous_moves_index() {
        let mut app = App::new();
        app.add_exchange(make_exchange());
        app.add_exchange(make_exchange());
        app.selected_index = 1;
        app.previous();
        assert_eq!(app.selected_index, 0);
        // Should not go below 0
        app.previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn next_on_empty_list() {
        let mut app = App::new();
        app.next();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn previous_at_zero() {
        let mut app = App::new();
        app.previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn add_exchange_updates_stats() {
        let mut app = App::new();
        app.add_exchange(make_exchange());
        assert_eq!(app.stats.total_requests, 1);
        assert_eq!(app.stats.total_responses, 1);
        assert_eq!(app.stats.bytes_captured, 8); // 3 (req) + 5 (resp)
        assert_eq!(app.exchanges.len(), 1);
    }

    #[test]
    fn add_exchange_no_response() {
        let mut app = App::new();
        let mut ex = make_exchange();
        ex.response = None;
        app.add_exchange(ex);
        assert_eq!(app.stats.total_requests, 1);
        assert_eq!(app.stats.total_responses, 0);
        assert_eq!(app.stats.bytes_captured, 3);
    }

    #[test]
    fn toggle_tab() {
        let mut app = App::new();
        assert_eq!(app.tab, AppTab::RequestList);
        app.toggle_tab();
        assert_eq!(app.tab, AppTab::Detail);
        app.toggle_tab();
        assert_eq!(app.tab, AppTab::RequestList);
    }

    #[test]
    fn selected_exchange_returns_correct() {
        let mut app = App::new();
        assert!(app.selected_exchange().is_none());
        app.add_exchange(make_exchange());
        assert!(app.selected_exchange().is_some());
    }
}
