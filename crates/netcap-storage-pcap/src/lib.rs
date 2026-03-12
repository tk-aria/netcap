pub mod converter;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use netcap_core::capture::exchange::CapturedExchange;
use netcap_core::error::StorageError;
use netcap_core::storage::StorageBackend;
use pcap_file::pcap::{PcapHeader, PcapPacket, PcapWriter};

pub struct PcapStorageConfig {
    pub output_path: PathBuf,
    pub snaplen: u32,
}

impl Default for PcapStorageConfig {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("capture.pcap"),
            snaplen: 65535,
        }
    }
}

pub struct PcapStorage {
    writer: Mutex<PcapWriter<std::fs::File>>,
    snaplen: u32,
}

impl PcapStorage {
    pub fn new(config: PcapStorageConfig) -> Result<Self, StorageError> {
        let file = std::fs::File::create(&config.output_path)
            .map_err(|e| StorageError::InitFailed(format!("Failed to create PCAP file: {}", e)))?;

        let header = PcapHeader {
            snaplen: config.snaplen,
            ..PcapHeader::default()
        };

        let writer = PcapWriter::with_header(file, header)
            .map_err(|e| StorageError::InitFailed(format!("Failed to write PCAP header: {}", e)))?;

        Ok(Self {
            writer: Mutex::new(writer),
            snaplen: config.snaplen,
        })
    }

    fn write_exchange_sync(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| StorageError::WriteFailed(format!("Lock poisoned: {}", e)))?;

        let timestamp = chrono_to_duration(&exchange.request.timestamp);

        // Write request packet
        let req_data = converter::build_request_packet(exchange);
        let orig_len = req_data.len() as u32;
        let snap_data = if orig_len > self.snaplen {
            &req_data[..self.snaplen as usize]
        } else {
            &req_data
        };
        let packet = PcapPacket::new(timestamp, orig_len, snap_data);
        writer
            .write_packet(&packet)
            .map_err(|e| StorageError::WriteFailed(format!("Failed to write request packet: {}", e)))?;

        // Write response packet if present
        if let Some(resp_data) = converter::build_response_packet(exchange) {
            let resp_ts = exchange
                .response
                .as_ref()
                .map(|r| chrono_to_duration(&r.timestamp))
                .unwrap_or(timestamp);
            let orig_len = resp_data.len() as u32;
            let snap_data = if orig_len > self.snaplen {
                &resp_data[..self.snaplen as usize]
            } else {
                &resp_data
            };
            let packet = PcapPacket::new(resp_ts, orig_len, snap_data);
            writer
                .write_packet(&packet)
                .map_err(|e| StorageError::WriteFailed(format!("Failed to write response packet: {}", e)))?;
        }

        Ok(())
    }
}

fn chrono_to_duration(dt: &chrono::DateTime<chrono::Utc>) -> Duration {
    let ts = dt.timestamp();
    let nanos = dt.timestamp_subsec_nanos();
    if ts >= 0 {
        Duration::new(ts as u64, nanos)
    } else {
        Duration::ZERO
    }
}

#[async_trait]
impl StorageBackend for PcapStorage {
    async fn initialize(&mut self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn write(&self, exchange: &CapturedExchange) -> Result<(), StorageError> {
        self.write_exchange_sync(exchange)
    }

    async fn write_batch(&self, exchanges: &[CapturedExchange]) -> Result<(), StorageError> {
        for exchange in exchanges {
            self.write_exchange_sync(exchange)?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), StorageError> {
        // File writes are unbuffered (no BufWriter), so flush is a no-op
        Ok(())
    }

    async fn close(&mut self) -> Result<(), StorageError> {
        self.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use chrono::Utc;
    use http::{HeaderMap, Method, StatusCode, Version};
    use netcap_core::capture::exchange::{CapturedRequest, CapturedResponse};
    use pcap_file::pcap::PcapReader;
    use uuid::Uuid;

    fn make_test_exchange(with_response: bool) -> CapturedExchange {
        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com".parse().unwrap());
        let req = CapturedRequest {
            id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            connection_id: Uuid::now_v7(),
            sequence_number: 0,
            timestamp: Utc::now(),
            method: Method::GET,
            uri: "http://example.com/test".parse().unwrap(),
            version: Version::HTTP_11,
            headers,
            body: Bytes::from("hello"),
            body_truncated: false,
            tls_info: None,
        };
        let response = if with_response {
            Some(CapturedResponse {
                id: Uuid::now_v7(),
                request_id: req.id,
                timestamp: Utc::now(),
                status: StatusCode::OK,
                version: Version::HTTP_11,
                headers: HeaderMap::new(),
                body: Bytes::from("world"),
                body_truncated: false,
                latency: std::time::Duration::from_millis(10),
                ttfb: std::time::Duration::from_millis(5),
            })
        } else {
            None
        };
        CapturedExchange {
            request: req,
            response,
        }
    }

    #[tokio::test]
    async fn new_creates_pcap_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.pcap");
        let config = PcapStorageConfig {
            output_path: path.clone(),
            snaplen: 65535,
        };
        let _storage = PcapStorage::new(config).unwrap();
        assert!(path.exists());
    }

    #[tokio::test]
    async fn write_single_exchange() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.pcap");
        let storage = PcapStorage::new(PcapStorageConfig {
            output_path: path.clone(),
            snaplen: 65535,
        })
        .unwrap();

        let exchange = make_test_exchange(true);
        storage.write(&exchange).await.unwrap();
        storage.flush().await.unwrap();

        // Verify file is readable
        let file = std::fs::File::open(&path).unwrap();
        let mut reader = PcapReader::new(file).unwrap();
        let mut count = 0;
        while let Some(pkt) = reader.next_packet() {
            pkt.unwrap();
            count += 1;
        }
        // Request + Response = 2 packets
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn write_request_only() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.pcap");
        let storage = PcapStorage::new(PcapStorageConfig {
            output_path: path.clone(),
            snaplen: 65535,
        })
        .unwrap();

        let exchange = make_test_exchange(false);
        storage.write(&exchange).await.unwrap();
        storage.flush().await.unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let mut reader = PcapReader::new(file).unwrap();
        let mut count = 0;
        while let Some(pkt) = reader.next_packet() {
            pkt.unwrap();
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn write_batch_multiple() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.pcap");
        let storage = PcapStorage::new(PcapStorageConfig {
            output_path: path.clone(),
            snaplen: 65535,
        })
        .unwrap();

        let exchanges = vec![
            make_test_exchange(true),
            make_test_exchange(true),
            make_test_exchange(false),
        ];
        storage.write_batch(&exchanges).await.unwrap();
        storage.flush().await.unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let mut reader = PcapReader::new(file).unwrap();
        let mut count = 0;
        while let Some(pkt) = reader.next_packet() {
            pkt.unwrap();
            count += 1;
        }
        // 2 with response (2 packets each) + 1 without (1 packet) = 5
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn pcap_packets_contain_http_data() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.pcap");
        let storage = PcapStorage::new(PcapStorageConfig {
            output_path: path.clone(),
            snaplen: 65535,
        })
        .unwrap();

        let exchange = make_test_exchange(true);
        storage.write(&exchange).await.unwrap();
        storage.flush().await.unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let mut reader = PcapReader::new(file).unwrap();

        // First packet = request
        let pkt = reader.next_packet().unwrap().unwrap();
        let data = String::from_utf8_lossy(&pkt.data);
        assert!(data.contains("GET /test HTTP/1.1"));

        // Second packet = response
        let pkt = reader.next_packet().unwrap().unwrap();
        let data = String::from_utf8_lossy(&pkt.data);
        assert!(data.contains("HTTP/1.1 200 OK"));
    }

    #[tokio::test]
    async fn invalid_path_returns_error() {
        let config = PcapStorageConfig {
            output_path: PathBuf::from("/nonexistent/dir/test.pcap"),
            snaplen: 65535,
        };
        let result = PcapStorage::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn snaplen_truncates_large_packets() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.pcap");
        // Very small snaplen
        let storage = PcapStorage::new(PcapStorageConfig {
            output_path: path.clone(),
            snaplen: 100,
        })
        .unwrap();

        let exchange = make_test_exchange(true);
        storage.write(&exchange).await.unwrap();
        storage.flush().await.unwrap();

        let file = std::fs::File::open(&path).unwrap();
        let mut reader = PcapReader::new(file).unwrap();
        let pkt = reader.next_packet().unwrap().unwrap();
        assert!(pkt.data.len() <= 100);
    }
}
