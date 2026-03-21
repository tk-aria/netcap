use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct OdooClient {
    http: reqwest::Client,
    base_url: String,
    db: String,
    uid: i64,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: i64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<Value>,
}

impl OdooClient {
    /// Authenticate and create a new OdooClient.
    pub async fn login(base_url: &str, db: &str, user: &str, password: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let resp: JsonRpcResponse = http
            .post(format!("{base_url}/jsonrpc"))
            .json(&JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "call".into(),
                params: json!({
                    "service": "common",
                    "method": "login",
                    "args": [db, user, password]
                }),
                id: 1,
            })
            .send()
            .await?
            .json()
            .await?;

        let uid = resp
            .result
            .as_ref()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow!("Login failed: {:?}", resp.error))?;

        tracing::info!("Authenticated as uid={uid}");

        Ok(Self {
            http,
            base_url: base_url.to_string(),
            db: db.to_string(),
            uid,
            password: password.to_string(),
        })
    }

    /// Call execute_kw on the Odoo object service.
    pub async fn execute_kw(
        &self,
        model: &str,
        method: &str,
        args: Value,
        kwargs: Option<Value>,
    ) -> Result<Value> {
        let mut call_args = vec![
            json!(self.db),
            json!(self.uid),
            json!(self.password),
            json!(model),
            json!(method),
            args,
        ];
        if let Some(kw) = kwargs {
            call_args.push(kw);
        }

        let resp: JsonRpcResponse = self
            .http
            .post(format!("{}/jsonrpc", self.base_url))
            .json(&JsonRpcRequest {
                jsonrpc: "2.0".into(),
                method: "call".into(),
                params: json!({
                    "service": "object",
                    "method": "execute_kw",
                    "args": call_args
                }),
                id: 1,
            })
            .send()
            .await?
            .json()
            .await?;

        if let Some(err) = resp.error {
            return Err(anyhow!("Odoo RPC error: {err}"));
        }

        resp.result.ok_or_else(|| anyhow!("Empty result from Odoo"))
    }

    /// Search records.
    pub async fn search(&self, model: &str, domain: Value) -> Result<Vec<i64>> {
        let result = self.execute_kw(model, "search", json!([domain]), None).await?;
        let ids: Vec<i64> = serde_json::from_value(result)?;
        Ok(ids)
    }

    /// Search and read records.
    pub async fn search_read(
        &self,
        model: &str,
        domain: Value,
        fields: &[&str],
    ) -> Result<Vec<Value>> {
        let result = self
            .execute_kw(
                model,
                "search_read",
                json!([domain]),
                Some(json!({"fields": fields})),
            )
            .await?;
        let records: Vec<Value> = serde_json::from_value(result)?;
        Ok(records)
    }

    /// Create a record and return its ID.
    pub async fn create(&self, model: &str, vals: Value) -> Result<i64> {
        let result = self.execute_kw(model, "create", json!([vals]), None).await?;
        result
            .as_i64()
            .ok_or_else(|| anyhow!("Expected integer ID from create, got: {result}"))
    }

    /// Write to existing records.
    pub async fn write(&self, model: &str, ids: &[i64], vals: Value) -> Result<()> {
        self.execute_kw(model, "write", json!([ids, vals]), None)
            .await?;
        Ok(())
    }

    /// Get the bank journal ID (type=bank).
    pub async fn get_bank_journal_id(&self) -> Result<i64> {
        let ids = self
            .search("account.journal", json!([["type", "=", "bank"]]))
            .await?;
        ids.into_iter()
            .next()
            .ok_or_else(|| anyhow!("No bank journal found"))
    }

    /// List existing bank statement lines for a journal.
    pub async fn list_statement_lines(
        &self,
        journal_id: i64,
        limit: Option<i64>,
    ) -> Result<Vec<Value>> {
        let result = self
            .execute_kw(
                "account.bank.statement.line",
                "search_read",
                json!([[
                    ["journal_id", "=", journal_id]
                ]]),
                Some(json!({
                    "fields": ["date", "payment_ref", "amount", "partner_id"],
                    "limit": limit.unwrap_or(20),
                    "order": "date desc"
                })),
            )
            .await?;
        let records: Vec<Value> = serde_json::from_value(result)?;
        Ok(records)
    }

    /// Create a bank statement line.
    pub async fn create_statement_line(
        &self,
        journal_id: i64,
        date: &str,
        payment_ref: &str,
        amount: f64,
        partner_id: Option<i64>,
    ) -> Result<i64> {
        let mut vals = json!({
            "journal_id": journal_id,
            "date": date,
            "payment_ref": payment_ref,
            "amount": amount,
        });

        if let Some(pid) = partner_id {
            vals["partner_id"] = json!(pid);
        }

        self.create("account.bank.statement.line", vals).await
    }

    /// Search for a partner by name.
    pub async fn find_partner_by_name(&self, name: &str) -> Result<Option<i64>> {
        let ids = self
            .search("res.partner", json!([["name", "ilike", name]]))
            .await?;
        Ok(ids.into_iter().next())
    }

    /// Get account journals summary (for dashboard display).
    pub async fn get_journals_summary(&self) -> Result<Vec<Value>> {
        self.search_read(
            "account.journal",
            json!([["type", "in", ["bank", "cash"]]]),
            &["name", "type", "bank_account_id", "current_statement_balance"],
        )
        .await
    }
}
