use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub session_id: Uuid,
    pub client_addr: SocketAddr,
    pub server_hostname: String,
    pub server_addr: Option<SocketAddr>,
    pub is_tls: bool,
    pub tls_version: Option<String>,
    pub cipher_suite: Option<String>,
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub established_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub close_reason: Option<String>,
    pub request_count: u64,
}

pub struct ConnectionTracker {
    connections: DashMap<Uuid, ConnectionInfo>,
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    pub fn track(&self, info: ConnectionInfo) -> Uuid {
        let id = info.id;
        self.connections.insert(id, info);
        id
    }

    pub fn increment_request_count(&self, id: &Uuid) {
        if let Some(mut conn) = self.connections.get_mut(id) {
            conn.request_count += 1;
        }
    }

    pub fn close(&self, id: &Uuid, reason: &str) {
        if let Some(mut conn) = self.connections.get_mut(id) {
            conn.closed_at = Some(Utc::now());
            conn.close_reason = Some(reason.to_string());
        }
    }

    pub fn get(&self, id: &Uuid) -> Option<ConnectionInfo> {
        self.connections.get(id).map(|c| c.clone())
    }

    pub fn active_count(&self) -> usize {
        self.connections
            .iter()
            .filter(|c| c.closed_at.is_none())
            .count()
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_conn_info(session_id: Uuid) -> ConnectionInfo {
        ConnectionInfo {
            id: Uuid::now_v7(),
            session_id,
            client_addr: "127.0.0.1:5000".parse().unwrap(),
            server_hostname: "example.com".to_string(),
            server_addr: Some("93.184.216.34:443".parse().unwrap()),
            is_tls: true,
            tls_version: Some("TLSv1.3".to_string()),
            cipher_suite: Some("TLS_AES_256_GCM_SHA384".to_string()),
            sni: Some("example.com".to_string()),
            alpn: Some("h2".to_string()),
            established_at: Utc::now(),
            closed_at: None,
            close_reason: None,
            request_count: 0,
        }
    }

    #[test]
    fn track_and_get() {
        let tracker = ConnectionTracker::new();
        let session_id = Uuid::now_v7();
        let info = make_conn_info(session_id);
        let id = info.id;
        tracker.track(info);
        let retrieved = tracker.get(&id).unwrap();
        assert_eq!(retrieved.server_hostname, "example.com");
        assert_eq!(retrieved.request_count, 0);
    }

    #[test]
    fn increment_request_count() {
        let tracker = ConnectionTracker::new();
        let info = make_conn_info(Uuid::now_v7());
        let id = info.id;
        tracker.track(info);
        tracker.increment_request_count(&id);
        tracker.increment_request_count(&id);
        let conn = tracker.get(&id).unwrap();
        assert_eq!(conn.request_count, 2);
    }

    #[test]
    fn close_connection() {
        let tracker = ConnectionTracker::new();
        let info = make_conn_info(Uuid::now_v7());
        let id = info.id;
        tracker.track(info);
        tracker.close(&id, "client disconnect");
        let conn = tracker.get(&id).unwrap();
        assert!(conn.closed_at.is_some());
        assert_eq!(conn.close_reason.as_deref(), Some("client disconnect"));
    }

    #[test]
    fn active_count() {
        let tracker = ConnectionTracker::new();
        let info1 = make_conn_info(Uuid::now_v7());
        let id1 = info1.id;
        let info2 = make_conn_info(Uuid::now_v7());
        tracker.track(info1);
        tracker.track(info2);
        assert_eq!(tracker.active_count(), 2);
        tracker.close(&id1, "done");
        assert_eq!(tracker.active_count(), 1);
    }

    #[test]
    fn nonexistent_connection_operations() {
        let tracker = ConnectionTracker::new();
        let fake_id = Uuid::now_v7();
        tracker.increment_request_count(&fake_id);
        tracker.close(&fake_id, "test");
        assert!(tracker.get(&fake_id).is_none());
    }
}
