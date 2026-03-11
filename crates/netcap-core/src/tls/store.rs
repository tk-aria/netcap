use dashmap::DashMap;
use std::time::{Duration, Instant};

use crate::tls::ServerCertificate;

struct CacheEntry {
    cert: ServerCertificate,
    created_at: Instant,
}

pub struct CertificateCache {
    cache: DashMap<String, CacheEntry>,
    ttl: Duration,
}

impl CertificateCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            ttl,
        }
    }

    pub fn get(&self, domain: &str) -> Option<ServerCertificate> {
        self.cache.get(domain).and_then(|entry| {
            if entry.created_at.elapsed() < self.ttl {
                Some(entry.cert.clone())
            } else {
                drop(entry);
                self.cache.remove(domain);
                None
            }
        })
    }

    pub fn insert(&self, domain: String, cert: ServerCertificate) {
        self.cache.insert(
            domain,
            CacheEntry {
                cert,
                created_at: Instant::now(),
            },
        );
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cert(domain: &str) -> ServerCertificate {
        ServerCertificate {
            cert_der: vec![1, 2, 3],
            key_der: vec![4, 5, 6],
            domain: domain.to_string(),
        }
    }

    #[test]
    fn insert_and_get() {
        let cache = CertificateCache::new(Duration::from_secs(3600));
        cache.insert("example.com".into(), make_cert("example.com"));
        let cert = cache.get("example.com").unwrap();
        assert_eq!(cert.domain, "example.com");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let cache = CertificateCache::new(Duration::from_secs(3600));
        assert!(cache.get("missing.com").is_none());
    }

    #[test]
    fn ttl_expiration() {
        let cache = CertificateCache::new(Duration::from_millis(1));
        cache.insert("example.com".into(), make_cert("example.com"));
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get("example.com").is_none());
    }

    #[test]
    fn clear_cache() {
        let cache = CertificateCache::new(Duration::from_secs(3600));
        cache.insert("a.com".into(), make_cert("a.com"));
        cache.insert("b.com".into(), make_cert("b.com"));
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn independent_domains() {
        let cache = CertificateCache::new(Duration::from_secs(3600));
        cache.insert("a.com".into(), make_cert("a.com"));
        cache.insert("b.com".into(), make_cert("b.com"));
        let a = cache.get("a.com").unwrap();
        let b = cache.get("b.com").unwrap();
        assert_eq!(a.domain, "a.com");
        assert_eq!(b.domain, "b.com");
    }

    #[test]
    fn overwrite_existing() {
        let cache = CertificateCache::new(Duration::from_secs(3600));
        cache.insert("example.com".into(), make_cert("example.com"));
        let new_cert = ServerCertificate {
            cert_der: vec![10, 20, 30],
            key_der: vec![40, 50, 60],
            domain: "example.com".into(),
        };
        cache.insert("example.com".into(), new_cert);
        let cert = cache.get("example.com").unwrap();
        assert_eq!(cert.cert_der, vec![10, 20, 30]);
    }
}
