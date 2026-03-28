use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use super::LogFile;

pub async fn serve(_port: u16, _root: Arc<PathBuf>, _log_file: LogFile) -> Result<()> {
    todo!("HTTP server not yet implemented")
}
