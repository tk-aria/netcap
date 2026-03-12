use anyhow::Result;
use std::sync::Arc;

use netcap_core::tls::ca::RcgenCaProvider;
use netcap_core::tls::CertificateProvider;

use crate::args::CertAction;

pub async fn execute(action: CertAction) -> Result<()> {
    match action {
        CertAction::Generate {
            common_name,
            output,
        } => {
            let parent = output
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            std::fs::create_dir_all(parent)?;

            let provider = RcgenCaProvider::generate_ca(&common_name, parent)?;
            let cert_path = output.with_extension("pem");
            let key_path = output.with_extension("key.pem");

            std::fs::write(&cert_path, provider.ca_cert_pem())?;
            std::fs::write(&key_path, provider.ca_key_pem())?;

            tracing::info!("CA certificate generated:");
            tracing::info!("  Certificate: {}", cert_path.display());
            tracing::info!("  Private key: {}", key_path.display());
            println!("CA certificate generated:");
            println!("  Certificate: {}", cert_path.display());
            println!("  Private key: {}", key_path.display());
            Ok(())
        }
        CertAction::Export { output } => {
            // Look for existing CA cert in default location
            let ca_dir = std::path::Path::new("netcap-ca");
            let cert_path = ca_dir.join("ca.pem");
            let key_path = ca_dir.join("ca.key.pem");

            if cert_path.exists() && key_path.exists() {
                let provider =
                    RcgenCaProvider::load_from_files(&cert_path, &key_path, ca_dir)?;
                let prov: Arc<dyn CertificateProvider> = Arc::new(provider);
                prov.export_ca_pem(&output).await?;
                println!("CA certificate exported to: {}", output.display());
            } else {
                // Generate a new one and export
                let provider = RcgenCaProvider::generate_ca("netcap CA", ca_dir)?;
                let prov: Arc<dyn CertificateProvider> = Arc::new(provider);
                prov.export_ca_pem(&output).await?;
                println!(
                    "No existing CA found. Generated and exported to: {}",
                    output.display()
                );
            }
            Ok(())
        }
    }
}
