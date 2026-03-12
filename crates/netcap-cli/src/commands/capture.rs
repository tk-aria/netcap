use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

use netcap_core::config::ProxyConfig;
use netcap_core::filter::pattern::DomainPattern;
use netcap_core::filter::{DomainFilter, DomainMatcher, FilterRule, FilterType};
use netcap_core::proxy::ProxyServer;
use netcap_core::storage::StorageBackend;
use netcap_core::tls::ca::RcgenCaProvider;
use uuid::Uuid;

use crate::args::StorageType;

async fn create_storage(
    storage_type: &StorageType,
    output_dir: &Path,
) -> Result<Arc<dyn StorageBackend>> {
    match storage_type {
        StorageType::Sqlite => {
            let db_path = output_dir.join("netcap.db");
            let config = netcap_storage_sqlite::SqliteStorageConfig {
                db_path: db_path.clone(),
            };
            let storage = netcap_storage_sqlite::SqliteStorage::new(config)?;
            tracing::info!("SQLite storage: {}", db_path.display());
            Ok(Arc::new(storage))
        }
        StorageType::Jsonl => {
            let jsonl_path = output_dir.join("netcap.jsonl");
            let config = netcap_storage_jsonl::JsonlStorageConfig {
                output_path: jsonl_path.clone(),
                rotate_size: Some(100 * 1024 * 1024),
            };
            let storage = netcap_storage_jsonl::JsonlStorage::new(config).await?;
            tracing::info!("JSONL storage: {}", jsonl_path.display());
            Ok(Arc::new(storage))
        }
        StorageType::Pcap => {
            let pcap_path = output_dir.join("netcap.pcap");
            let config = netcap_storage_pcap::PcapStorageConfig {
                output_path: pcap_path.clone(),
                snaplen: 65535,
            };
            let storage = netcap_storage_pcap::PcapStorage::new(config)?;
            tracing::info!("PCAP storage: {}", pcap_path.display());
            Ok(Arc::new(storage))
        }
    }
}

pub async fn execute(
    listen: &str,
    include_domains: &[String],
    exclude_domains: &[String],
    storage_types: &[StorageType],
    output_dir: &Path,
) -> Result<()> {
    // 1. Prepare CA certificate
    let ca_path = output_dir.join("netcap-ca");
    std::fs::create_dir_all(&ca_path)?;
    let ca_provider = Arc::new(RcgenCaProvider::generate_ca("netcap CA", &ca_path)?);
    tracing::info!("CA certificate generated");

    // 2. Domain filter setup
    let mut filter = DomainFilter::new();
    for domain in include_domains {
        filter.add_rule(FilterRule {
            id: Uuid::now_v7(),
            name: format!("include:{}", domain),
            filter_type: FilterType::Include,
            pattern: DomainPattern::new_wildcard(domain),
            priority: 100,
            enabled: true,
        });
    }
    for domain in exclude_domains {
        filter.add_rule(FilterRule {
            id: Uuid::now_v7(),
            name: format!("exclude:{}", domain),
            filter_type: FilterType::Exclude,
            pattern: DomainPattern::new_wildcard(domain),
            priority: 200,
            enabled: true,
        });
    }

    // 3. Storage initialization (use first storage type)
    let storage_type = storage_types.first().unwrap_or(&StorageType::Sqlite);
    let storage = create_storage(storage_type, output_dir).await?;

    // 4. Build ProxyServer
    let config = ProxyConfig {
        listen_addr: listen.parse()?,
        ..Default::default()
    };

    let server = Arc::new(
        ProxyServer::builder()
            .config(config)
            .cert_provider(ca_provider)
            .domain_filter(Arc::new(filter))
            .storage(storage)
            .build()?,
    );

    println!("Proxy listening on {}", listen);

    // 5. Ctrl+C graceful shutdown
    let server_clone = Arc::clone(&server);
    tokio::select! {
        result = server_clone.run() => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutting down...");
            server.shutdown()?;
        }
    }

    Ok(())
}
