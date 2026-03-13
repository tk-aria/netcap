use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 100;

pub async fn retry_with_backoff<F, Fut, T, E>(mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= MAX_RETRIES {
                    tracing::error!(
                        "BigQuery write failed after {} retries: {}",
                        MAX_RETRIES,
                        e
                    );
                    return Err(e);
                }
                let delay = Duration::from_millis(BASE_DELAY_MS * 2u64.pow(attempt - 1));
                tracing::warn!(
                    "BigQuery write attempt {} failed: {}. Retrying in {:?}",
                    attempt,
                    e,
                    delay
                );
                sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn retry_succeeds_first_attempt() {
        let result: Result<i32, String> = retry_with_backoff(|| async { Ok(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn retry_succeeds_after_failures() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        let result: Result<i32, String> = retry_with_backoff(move || {
            let c = counter_clone.clone();
            async move {
                let attempt = c.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(format!("fail {}", attempt))
                } else {
                    Ok(99)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_exhausted_returns_error() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        let result: Result<i32, String> = retry_with_backoff(move || {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err("always fail".to_string())
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
