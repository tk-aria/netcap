use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct JsonlEntry {
    method: String,
    uri: String,
    #[serde(default)]
    headers: Vec<(String, String)>,
    #[serde(default)]
    body: Option<String>,
}

pub async fn execute(input: &Path, target_base: Option<&str>) -> Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let entries = match ext {
        "jsonl" => load_from_jsonl(input)?,
        "db" | "sqlite" => load_from_sqlite(input)?,
        _ => anyhow::bail!("Unsupported file format: {}. Use .jsonl or .db", ext),
    };

    if entries.is_empty() {
        println!("No requests found in {}", input.display());
        return Ok(());
    }

    println!("Replaying {} requests from {}", entries.len(), input.display());

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    for (i, entry) in entries.iter().enumerate() {
        let uri = if let Some(base) = target_base {
            // Replace host with target base
            if let Ok(parsed) = entry.uri.parse::<http::Uri>() {
                let path_and_query = parsed.path_and_query()
                    .map(|pq| pq.as_str())
                    .unwrap_or("/");
                format!("{}{}", base.trim_end_matches('/'), path_and_query)
            } else {
                format!("{}{}", base.trim_end_matches('/'), &entry.uri)
            }
        } else {
            entry.uri.clone()
        };

        let method: reqwest::Method = entry.method.parse().unwrap_or(reqwest::Method::GET);
        let mut req = client.request(method.clone(), &uri);

        for (key, value) in &entry.headers {
            // Skip host header as reqwest sets it
            if key.to_lowercase() != "host" {
                req = req.header(key.as_str(), value.as_str());
            }
        }

        if let Some(ref body) = entry.body {
            req = req.body(body.clone());
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                println!(
                    "[{}/{}] {} {} → {}",
                    i + 1,
                    entries.len(),
                    entry.method,
                    uri,
                    status
                );
            }
            Err(e) => {
                println!(
                    "[{}/{}] {} {} → ERROR: {}",
                    i + 1,
                    entries.len(),
                    entry.method,
                    uri,
                    e
                );
            }
        }
    }

    println!("Replay complete.");
    Ok(())
}

fn load_from_jsonl(path: &Path) -> Result<Vec<JsonlEntry>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut entries = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: JsonlEntry = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse line {} in {}", i + 1, path.display()))?;
        entries.push(entry);
    }
    Ok(entries)
}

fn load_from_sqlite(path: &Path) -> Result<Vec<JsonlEntry>> {
    if !path.exists() {
        anyhow::bail!("Database file not found: {}", path.display());
    }
    // Use netcap-storage-sqlite to query
    let conn = rusqlite::Connection::open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;
    let mut stmt = conn
        .prepare("SELECT method, uri, request_headers, request_body FROM requests ORDER BY timestamp")
        .with_context(|| "Failed to query requests table")?;

    let entries: Vec<JsonlEntry> = stmt
        .query_map([], |row| {
            let method: String = row.get(0)?;
            let uri: String = row.get(1)?;
            let headers_json: Option<String> = row.get(2)?;
            let body: Option<String> = row.get(3)?;
            let headers: Vec<(String, String)> = headers_json
                .and_then(|h| serde_json::from_str(&h).ok())
                .unwrap_or_default();
            Ok(JsonlEntry {
                method,
                uri,
                headers,
                body,
            })
        })
        .with_context(|| "Failed to iterate rows")?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_jsonl_valid() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp.as_file(),
            r#"{{"method":"GET","uri":"http://example.com/api","headers":[]}}"#
        )
        .unwrap();
        writeln!(
            tmp.as_file(),
            r#"{{"method":"POST","uri":"http://example.com/data","headers":[],"body":"hello"}}"#
        )
        .unwrap();

        let entries = load_from_jsonl(tmp.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].method, "GET");
        assert_eq!(entries[1].method, "POST");
        assert_eq!(entries[1].body, Some("hello".into()));
    }

    #[test]
    fn load_jsonl_nonexistent_file() {
        let result = load_from_jsonl(Path::new("/nonexistent/file.jsonl"));
        assert!(result.is_err());
    }

    #[test]
    fn load_sqlite_nonexistent_file() {
        let result = load_from_sqlite(Path::new("/nonexistent/db.db"));
        assert!(result.is_err());
    }

    #[test]
    fn load_jsonl_empty() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let entries = load_from_jsonl(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }
}
