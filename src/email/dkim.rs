use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::CheckResult;

pub async fn check(_resolver: &TokioAsyncResolver, _host: &str, _selector: &str) -> Result<CheckResult> {
    todo!("DKIM check not yet implemented")
}
