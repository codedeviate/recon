use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use super::LogFile;

pub async fn serve(_port: u16, _root: Arc<PathBuf>, _log_file: LogFile, _cert_path: &Path, _key_path: &Path, _http_version: Option<&str>) -> Result<()> {
    todo!("HTTPS server not yet implemented")
}
