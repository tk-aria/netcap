use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "netcap", version, about = "Cross-platform HTTP/HTTPS capture tool")]
pub struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "netcap.toml")]
    pub config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short = 'v', long, default_value = "info")]
    pub verbose: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start HTTP/HTTPS capture
    Capture {
        /// Listen address
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        listen: String,

        /// Domain filter (include)
        #[arg(short = 'i', long = "include", value_delimiter = ',')]
        include_domains: Vec<String>,

        /// Domain filter (exclude)
        #[arg(short = 'e', long = "exclude", value_delimiter = ',')]
        exclude_domains: Vec<String>,

        /// Storage backends
        #[arg(short, long, value_enum, default_value = "sqlite")]
        storage: Vec<StorageType>,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,
    },
    /// CA certificate management
    Cert {
        #[command(subcommand)]
        action: CertAction,
    },
    /// Replay captured requests
    Replay {
        /// Input file (.jsonl or .db)
        #[arg(short = 'f', long)]
        input: PathBuf,

        /// Target base URL (replaces original host)
        #[arg(short, long)]
        target: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CertAction {
    /// Generate a new CA certificate
    Generate {
        /// Common name for the CA
        #[arg(short = 'n', long, default_value = "netcap CA")]
        common_name: String,
        /// Output path for the certificate
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Export existing CA certificate in PEM format
    Export {
        /// Output path
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Clone, ValueEnum, Debug, PartialEq)]
pub enum StorageType {
    Sqlite,
    Jsonl,
    Pcap,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn capture_default_args_parse() {
        let cli = Cli::try_parse_from(["netcap", "capture"]).unwrap();
        match cli.command {
            Commands::Capture { listen, storage, .. } => {
                assert_eq!(listen, "127.0.0.1:8080");
                assert_eq!(storage, vec![StorageType::Sqlite]);
            }
            _ => panic!("Expected Capture command"),
        }
    }

    #[test]
    fn capture_multiple_include_domains() {
        let cli = Cli::try_parse_from([
            "netcap", "capture", "-i", "example.com,api.test.com",
        ])
        .unwrap();
        match cli.command {
            Commands::Capture { include_domains, .. } => {
                assert_eq!(include_domains, vec!["example.com", "api.test.com"]);
            }
            _ => panic!("Expected Capture command"),
        }
    }

    #[test]
    fn cert_generate_parses() {
        let cli = Cli::try_parse_from(["netcap", "cert", "generate", "-o", "./ca.pem"]).unwrap();
        match cli.command {
            Commands::Cert {
                action: CertAction::Generate { output, .. },
            } => {
                assert_eq!(output, std::path::PathBuf::from("./ca.pem"));
            }
            _ => panic!("Expected Cert Generate command"),
        }
    }

    #[test]
    fn unknown_subcommand_errors() {
        let result = Cli::try_parse_from(["netcap", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn cert_export_missing_output_errors() {
        let result = Cli::try_parse_from(["netcap", "cert", "export"]);
        assert!(result.is_err());
    }
}
