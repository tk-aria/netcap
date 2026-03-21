mod csv_parser;
mod odoo;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "odoo-bank-sync", about = "Odoo bank statement sync tool via JSON-RPC API")]
struct Cli {
    /// Odoo server URL
    #[arg(long, env = "ODOO_URL", default_value = "http://odoo.odoo.svc.cluster.local")]
    url: String,

    /// Database name
    #[arg(long, env = "ODOO_DB", default_value = "odoo")]
    db: String,

    /// Login user
    #[arg(long, env = "ODOO_USER", default_value = "admin")]
    user: String,

    /// Login password
    #[arg(long, env = "ODOO_PASSWORD", default_value = "admin")]
    password: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import bank transactions from a CSV file
    Import {
        /// Path to the CSV file
        #[arg(short, long)]
        file: PathBuf,

        /// Bank journal ID (auto-detected if omitted)
        #[arg(short, long)]
        journal_id: Option<i64>,

        /// CSV format preset
        #[arg(long, default_value = "mufg")]
        format: CsvFormat,

        /// Custom column name for date
        #[arg(long)]
        col_date: Option<String>,

        /// Custom column name for description
        #[arg(long)]
        col_desc: Option<String>,

        /// Custom column name for amount (single column)
        #[arg(long)]
        col_amount: Option<String>,

        /// Custom column name for deposit
        #[arg(long)]
        col_deposit: Option<String>,

        /// Custom column name for withdrawal
        #[arg(long)]
        col_withdrawal: Option<String>,

        /// Dry run - parse and display without importing
        #[arg(long)]
        dry_run: bool,
    },

    /// List recent bank statement lines
    List {
        /// Bank journal ID (auto-detected if omitted)
        #[arg(short, long)]
        journal_id: Option<i64>,

        /// Max number of lines to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },

    /// Show bank journals summary
    Journals,

    /// Register a bank and bank account, link to journal
    RegisterBank {
        /// Bank name (e.g. 三菱UFJ銀行)
        #[arg(long)]
        bank_name: String,
        /// BIC/SWIFT code
        #[arg(long, default_value = "")]
        bic: String,
        /// Account number
        #[arg(long)]
        account_number: String,
    },

    /// Run full demo: register bank + add sample transactions + show status
    Demo,

    /// Create a single bank statement line
    Add {
        /// Transaction date (YYYY-MM-DD)
        #[arg(short, long)]
        date: String,

        /// Payment reference / description
        #[arg(short, long)]
        reference: String,

        /// Amount (positive=deposit, negative=withdrawal)
        #[arg(short, long)]
        amount: f64,

        /// Bank journal ID (auto-detected if omitted)
        #[arg(short, long)]
        journal_id: Option<i64>,

        /// Partner name (will search in Odoo)
        #[arg(short, long)]
        partner: Option<String>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum CsvFormat {
    /// 三菱UFJ銀行 internet banking CSV
    Mufg,
    /// Generic format with single amount column
    Generic,
    /// Custom column mapping (use --col-* options)
    Custom,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("odoo_bank_sync=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    let client = odoo::OdooClient::login(&cli.url, &cli.db, &cli.user, &cli.password).await?;

    match cli.command {
        Command::Import {
            file,
            journal_id,
            format,
            col_date,
            col_desc,
            col_amount,
            col_deposit,
            col_withdrawal,
            dry_run,
        } => {
            let journal_id = match journal_id {
                Some(id) => id,
                None => client.get_bank_journal_id().await?,
            };

            let mapping = match format {
                CsvFormat::Mufg => {
                    let mut m = csv_parser::ColumnMap::mufg();
                    apply_overrides(&mut m, col_date, col_desc, col_amount, col_deposit, col_withdrawal);
                    m
                }
                CsvFormat::Generic => {
                    let mut m = csv_parser::ColumnMap::generic();
                    apply_overrides(&mut m, col_date, col_desc, col_amount, col_deposit, col_withdrawal);
                    m
                }
                CsvFormat::Custom => csv_parser::ColumnMap {
                    date: col_date.unwrap_or_else(|| "date".into()),
                    description: col_desc.unwrap_or_else(|| "description".into()),
                    deposit: col_deposit,
                    withdrawal: col_withdrawal,
                    amount: col_amount.or_else(|| Some("amount".into())),
                    partner: None,
                },
            };

            let transactions = csv_parser::parse_csv(&file, &mapping)?;

            if transactions.is_empty() {
                println!("No transactions found in CSV.");
                return Ok(());
            }

            println!("Parsed {} transactions:", transactions.len());
            println!("{:<12} {:<40} {:>12}", "Date", "Description", "Amount");
            println!("{}", "-".repeat(66));

            for tx in &transactions {
                println!(
                    "{:<12} {:<40} {:>12.0}",
                    tx.date,
                    truncate(&tx.description, 38),
                    tx.amount
                );
            }

            if dry_run {
                println!("\n[Dry run] No data was imported.");
                return Ok(());
            }

            println!("\nImporting to Odoo (journal_id={journal_id})...");
            let mut imported = 0;
            let mut errors = 0;

            for tx in &transactions {
                let partner_id = if let Some(ref name) = tx.partner {
                    client.find_partner_by_name(name).await.unwrap_or(None)
                } else {
                    None
                };

                match client
                    .create_statement_line(
                        journal_id,
                        &tx.date,
                        &tx.description,
                        tx.amount,
                        partner_id,
                    )
                    .await
                {
                    Ok(id) => {
                        imported += 1;
                        tracing::info!("Created statement line id={id}: {} {}", tx.date, tx.amount);
                    }
                    Err(e) => {
                        errors += 1;
                        tracing::error!("Failed to import: {} {} - {e}", tx.date, tx.description);
                    }
                }
            }

            println!("\nImport complete: {imported} imported, {errors} errors");
        }

        Command::List { journal_id, limit } => {
            let journal_id = match journal_id {
                Some(id) => id,
                None => client.get_bank_journal_id().await?,
            };

            let lines = client.list_statement_lines(journal_id, Some(limit)).await?;

            if lines.is_empty() {
                println!("No statement lines found.");
                return Ok(());
            }

            println!("{:<12} {:<40} {:>12} {}", "Date", "Reference", "Amount", "Partner");
            println!("{}", "-".repeat(80));

            for line in &lines {
                let date = line["date"].as_str().unwrap_or("-");
                let reference = line["payment_ref"]
                    .as_str()
                    .or_else(|| line["payment_ref"].as_bool().and_then(|b| if b { Some("") } else { Some("") }))
                    .unwrap_or("-");
                let amount = line["amount"].as_f64().unwrap_or(0.0);
                let partner = line["partner_id"]
                    .as_array()
                    .and_then(|a| a.get(1))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");

                println!(
                    "{:<12} {:<40} {:>12.0} {}",
                    date,
                    truncate(reference, 38),
                    amount,
                    partner
                );
            }
        }

        Command::RegisterBank {
            bank_name,
            bic,
            account_number,
        } => {
            register_bank(&client, &bank_name, &bic, &account_number).await?;
        }

        Command::Demo => {
            run_full_demo(&client).await?;
        }

        Command::Journals => {
            let journals = client.get_journals_summary().await?;

            println!("{:<4} {:<20} {:<8} {}", "ID", "Name", "Type", "Bank Account");
            println!("{}", "-".repeat(60));

            for j in &journals {
                let id = j["id"].as_i64().unwrap_or(0);
                let name = j["name"].as_str().unwrap_or("-");
                let jtype = j["type"].as_str().unwrap_or("-");
                let bank = j["bank_account_id"]
                    .as_array()
                    .and_then(|a| a.get(1))
                    .and_then(|v| v.as_str())
                    .unwrap_or("(none)");

                println!("{:<4} {:<20} {:<8} {}", id, name, jtype, bank);
            }
        }

        Command::Add {
            date,
            reference,
            amount,
            journal_id,
            partner,
        } => {
            let journal_id = match journal_id {
                Some(id) => id,
                None => client.get_bank_journal_id().await?,
            };

            let partner_id = if let Some(ref name) = partner {
                client.find_partner_by_name(name).await?
            } else {
                None
            };

            let id = client
                .create_statement_line(journal_id, &date, &reference, amount, partner_id)
                .await?;

            println!("Created statement line: id={id}");
            println!("  Date:      {date}");
            println!("  Reference: {reference}");
            println!("  Amount:    {amount:.0}");
            if let Some(pid) = partner_id {
                println!("  Partner:   id={pid}");
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{truncated}…")
    }
}

async fn register_bank(
    client: &odoo::OdooClient,
    bank_name: &str,
    bic: &str,
    account_number: &str,
) -> Result<()> {
    // 1. Find or create the bank (res.bank)
    let existing = client
        .search("res.bank", serde_json::json!([["name", "=", bank_name]]))
        .await?;

    let bank_id = if let Some(id) = existing.into_iter().next() {
        println!("Bank already exists: {} (id={})", bank_name, id);
        id
    } else {
        let mut vals = serde_json::json!({"name": bank_name});
        if !bic.is_empty() {
            vals["bic"] = serde_json::json!(bic);
        }
        let id = client.create("res.bank", vals).await?;
        println!("Created bank: {} (id={})", bank_name, id);
        id
    };

    // 2. Find or create bank account (res.partner.bank)
    let existing_acc = client
        .search(
            "res.partner.bank",
            serde_json::json!([["acc_number", "=", account_number]]),
        )
        .await?;

    let acc_id = if let Some(id) = existing_acc.into_iter().next() {
        println!("Bank account already exists: {} (id={})", account_number, id);
        id
    } else {
        let id = client
            .create(
                "res.partner.bank",
                serde_json::json!({
                    "acc_number": account_number,
                    "bank_id": bank_id,
                    "partner_id": 1,
                }),
            )
            .await?;
        println!("Created bank account: {} (id={})", account_number, id);
        id
    };

    // 3. Link to bank journal
    let journal_id = client.get_bank_journal_id().await?;
    let journals = client
        .search_read(
            "account.journal",
            serde_json::json!([["id", "=", journal_id]]),
            &["bank_account_id"],
        )
        .await?;

    let current_bank_acc = journals
        .first()
        .and_then(|j| j["bank_account_id"].as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_i64());

    if current_bank_acc == Some(acc_id) {
        println!("Bank journal already linked to account {}", account_number);
    } else {
        client
            .write(
                "account.journal",
                &[journal_id],
                serde_json::json!({"bank_account_id": acc_id}),
            )
            .await?;
        println!("Linked bank account {} to journal id={}", account_number, journal_id);
    }

    println!("Bank registration complete.");
    Ok(())
}

async fn run_full_demo(client: &odoo::OdooClient) -> Result<()> {
    println!("========================================");
    println!("  Odoo Bank Sync - Full Demo");
    println!("========================================\n");

    // Step 1: Register bank
    println!("--- Step 1: Bank Registration ---");
    register_bank(client, "三菱UFJ銀行", "BOTKJPJT", "普通 1234567").await?;

    // Step 2: Add demo transactions
    println!("\n--- Step 2: Add Demo Transactions ---");
    let journal_id = client.get_bank_journal_id().await?;
    let demo_lines = [
        ("2026-03-01", "売上入金 - 株式会社A", 500_000.0),
        ("2026-03-05", "仕入支払 - 株式会社B", -120_000.0),
        ("2026-03-10", "給与振込", -350_000.0),
        ("2026-03-15", "売上入金 - 株式会社C", 280_000.0),
        ("2026-03-20", "オフィス賃料", -150_000.0),
    ];

    for (date, ref_text, amount) in &demo_lines {
        let id = client
            .create_statement_line(journal_id, date, ref_text, *amount, None)
            .await?;
        let direction = if *amount >= 0.0 { "入金" } else { "出金" };
        println!(
            "  Created: {} {} ¥{:>10} - {} (id={})",
            date,
            direction,
            amount.abs() as i64,
            ref_text,
            id
        );
    }
    println!("{} demo transactions created.", demo_lines.len());

    // Step 3: Show status
    println!("\n--- Step 3: Current Status ---");
    let journals = client.get_journals_summary().await?;
    for j in &journals {
        let name = j["name"].as_str().unwrap_or("?");
        let jtype = j["type"].as_str().unwrap_or("?");
        let bank = j["bank_account_id"]
            .as_array()
            .and_then(|a| a.get(1))
            .and_then(|v| v.as_str())
            .unwrap_or("(none)");
        println!("  [{jtype}] {name} - {bank}");
    }

    let lines = client.list_statement_lines(journal_id, Some(10)).await?;
    println!("\nRecent transactions:");
    for line in &lines {
        let date = line["date"].as_str().unwrap_or("?");
        let ref_text = line["payment_ref"].as_str().unwrap_or("");
        let amount = line["amount"].as_f64().unwrap_or(0.0);
        let direction = if amount >= 0.0 { "入金" } else { "出金" };
        println!("  {} {} ¥{:>10} - {}", date, direction, amount.abs() as i64, ref_text);
    }

    println!("\n========================================");
    println!("  Demo complete!");
    println!("========================================");
    Ok(())
}

fn apply_overrides(
    m: &mut csv_parser::ColumnMap,
    date: Option<String>,
    desc: Option<String>,
    amount: Option<String>,
    deposit: Option<String>,
    withdrawal: Option<String>,
) {
    if let Some(v) = date {
        m.date = v;
    }
    if let Some(v) = desc {
        m.description = v;
    }
    if let Some(v) = amount {
        m.amount = Some(v);
        m.deposit = None;
        m.withdrawal = None;
    }
    if let Some(v) = deposit {
        m.deposit = Some(v);
    }
    if let Some(v) = withdrawal {
        m.withdrawal = Some(v);
    }
}
