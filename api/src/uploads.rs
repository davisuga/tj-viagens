use tokio::fs;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// Saves multipart bytes under a uuid-prefixed name. Returns (original_name, path).
pub async fn save_upload(dir: &str, original_name: &str, bytes: &[u8]) -> ApiResult<(String, String)> {
    // Untrusted client filename: keep only the final path component.
    let file_name = std::path::Path::new(original_name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty() && *n != "." && *n != "..")
        .unwrap_or("upload.bin")
        .to_string();
    fs::create_dir_all(dir).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let file_path = format!("{dir}/{}-{file_name}", Uuid::new_v4());
    fs::write(&file_path, bytes).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((file_name, file_path))
}
