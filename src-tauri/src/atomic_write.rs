//! Atomic write for config files: write to a temp file then rename to avoid corruption on crash.

use anyhow::Result;
use std::path::Path;

/// Write `contents` to `path` atomically (write to path.tmp then rename).
pub async fn atomic_write_async(path: &Path, contents: &str) -> Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file.tmp".into());
    let tmp = parent.join(format!("{}.tmp", name));
    tokio::fs::write(&tmp, contents).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}
