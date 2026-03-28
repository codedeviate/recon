use anyhow::Result;
use hickory_resolver::TokioAsyncResolver;
use super::CheckResult;

pub async fn check(_resolver: &TokioAsyncResolver, _host: &str) -> Result<CheckResult> {
    todo!("SPF check not yet implemented")
}
