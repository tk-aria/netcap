use std::path::Path;
use chrono::Utc;

/// Rotate the file at `path` by renaming it with a timestamp suffix,
/// then creating a new empty file at the original path.
///
/// The rotated file is named like `filename.jsonl.20260312_153000`.
pub async fn rotate(path: &Path) -> Result<(), std::io::Error> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();

    let file_name = path
        .to_str()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?;

    let rotated_path = format!("{}.{}", file_name, timestamp);

    tokio::fs::rename(path, &rotated_path).await?;
    tokio::fs::File::create(path).await?;

    tracing::info!(
        original = %path.display(),
        rotated = %rotated_path,
        "rotated JSONL file"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn rotate_renames_file_and_creates_new() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("capture.jsonl");

        // Write some content to the original file
        tokio::fs::write(&file_path, b"line1\nline2\n").await.unwrap();

        rotate(&file_path).await.unwrap();

        // Original path should exist and be empty (newly created)
        let new_content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert!(new_content.is_empty());

        // There should be a rotated file in the same directory
        let mut entries = tokio::fs::read_dir(dir.path()).await.unwrap();
        let mut rotated_found = false;
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("capture.jsonl.") && name != "capture.jsonl" {
                rotated_found = true;
                let content = tokio::fs::read_to_string(entry.path()).await.unwrap();
                assert_eq!(content, "line1\nline2\n");
            }
        }
        assert!(rotated_found, "rotated file should exist");
    }

    #[tokio::test]
    async fn rotate_nonexistent_file_fails() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("nonexistent.jsonl");
        let result = rotate(&file_path).await;
        assert!(result.is_err());
    }
}
