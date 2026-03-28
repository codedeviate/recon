use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::CheckResult;

pub async fn check(_resolver: &TokioAsyncResolver, _host: &str, _selector: &str, _insecure: bool) -> Result<CheckResult> {
    todo!("BIMI check not yet implemented")
}
