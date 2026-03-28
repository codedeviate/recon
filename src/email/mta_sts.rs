use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::CheckResult;

pub async fn check(_resolver: &TokioAsyncResolver, _host: &str, _insecure: bool) -> Result<CheckResult> {
    todo!("MTA-STS check not yet implemented")
}
