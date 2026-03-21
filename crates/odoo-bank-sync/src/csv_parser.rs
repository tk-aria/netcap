use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// A bank transaction parsed from CSV.
#[derive(Debug, Clone, Deserialize)]
pub struct BankTransaction {
    /// Transaction date (YYYY-MM-DD or YYYY/MM/DD)
    pub date: String,
    /// Description / payment reference
    pub description: String,
    /// Amount (positive=deposit, negative=withdrawal)
    pub amount: f64,
    /// Optional partner/payee name
    pub partner: Option<String>,
}

/// Parse a generic CSV file with configurable column mapping.
///
/// Expected CSV columns (configurable via ColumnMap):
/// - date column
/// - description column
/// - deposit/withdrawal or single amount column
pub fn parse_csv(path: &Path, mapping: &ColumnMap) -> Result<Vec<BankTransaction>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("Failed to open CSV: {}", path.display()))?;

    let headers = rdr.headers()?.clone();
    tracing::info!("CSV headers: {:?}", headers);

    let date_idx = find_column(&headers, &mapping.date)?;
    let desc_idx = find_column(&headers, &mapping.description)?;
    let deposit_idx = mapping
        .deposit
        .as_ref()
        .map(|c| find_column(&headers, c))
        .transpose()?;
    let withdrawal_idx = mapping
        .withdrawal
        .as_ref()
        .map(|c| find_column(&headers, c))
        .transpose()?;
    let amount_idx = mapping
        .amount
        .as_ref()
        .map(|c| find_column(&headers, c))
        .transpose()?;
    let partner_idx = mapping
        .partner
        .as_ref()
        .map(|c| find_column(&headers, c))
        .transpose()?;

    let mut transactions = Vec::new();

    for (i, record) in rdr.records().enumerate() {
        let record = record.with_context(|| format!("Failed to read CSV row {}", i + 1))?;

        let raw_date = record.get(date_idx).unwrap_or("").trim().to_string();
        let date = normalize_date(&raw_date);
        let description = record.get(desc_idx).unwrap_or("").trim().to_string();

        let amount = if let Some(aidx) = amount_idx {
            parse_amount(record.get(aidx).unwrap_or("0"))
        } else {
            let deposit = deposit_idx
                .map(|idx| parse_amount(record.get(idx).unwrap_or("0")))
                .unwrap_or(0.0);
            let withdrawal = withdrawal_idx
                .map(|idx| parse_amount(record.get(idx).unwrap_or("0")))
                .unwrap_or(0.0);
            deposit - withdrawal
        };

        if amount == 0.0 && description.is_empty() {
            continue; // skip empty rows
        }

        let partner = partner_idx
            .and_then(|idx| record.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        transactions.push(BankTransaction {
            date,
            description,
            amount,
            partner,
        });
    }

    tracing::info!("Parsed {} transactions from CSV", transactions.len());
    Ok(transactions)
}

/// Column mapping configuration for CSV parsing.
#[derive(Debug, Clone)]
pub struct ColumnMap {
    pub date: String,
    pub description: String,
    pub deposit: Option<String>,
    pub withdrawal: Option<String>,
    pub amount: Option<String>,
    pub partner: Option<String>,
}

impl Default for ColumnMap {
    fn default() -> Self {
        Self {
            date: "日付".into(),
            description: "摘要".into(),
            deposit: Some("お預り金額".into()),
            withdrawal: Some("お引出金額".into()),
            amount: None,
            partner: None,
        }
    }
}

impl ColumnMap {
    /// MUFG (三菱UFJ銀行) internet banking CSV format.
    pub fn mufg() -> Self {
        Self {
            date: "日付".into(),
            description: "摘要".into(),
            deposit: Some("お預り金額".into()),
            withdrawal: Some("お引出金額".into()),
            amount: None,
            partner: None,
        }
    }

    /// Generic single-amount format.
    pub fn generic() -> Self {
        Self {
            date: "date".into(),
            description: "description".into(),
            deposit: None,
            withdrawal: None,
            amount: Some("amount".into()),
            partner: Some("partner".into()),
        }
    }
}

fn find_column(headers: &csv::StringRecord, name: &str) -> Result<usize> {
    headers
        .iter()
        .position(|h| h.trim() == name)
        .ok_or_else(|| {
            let available: Vec<&str> = headers.iter().collect();
            anyhow::anyhow!("Column '{}' not found. Available: {:?}", name, available)
        })
}

fn parse_amount(s: &str) -> f64 {
    let cleaned: String = s
        .trim()
        .replace(',', "")
        .replace('¥', "")
        .replace('￥', "")
        .replace('\u{00a5}', ""); // yen sign
    cleaned.parse::<f64>().unwrap_or(0.0)
}

fn normalize_date(s: &str) -> String {
    // Handle YYYY/MM/DD -> YYYY-MM-DD
    let s = s.replace('/', "-");
    // Handle YYYYMMDD -> YYYY-MM-DD
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_amount() {
        assert_eq!(parse_amount("1,000"), 1000.0);
        assert_eq!(parse_amount("¥50,000"), 50000.0);
        assert_eq!(parse_amount(""), 0.0);
        assert_eq!(parse_amount("-3000"), -3000.0);
    }

    #[test]
    fn test_normalize_date() {
        assert_eq!(normalize_date("2026/03/21"), "2026-03-21");
        assert_eq!(normalize_date("20260321"), "2026-03-21");
        assert_eq!(normalize_date("2026-03-21"), "2026-03-21");
    }
}
