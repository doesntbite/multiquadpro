use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::Client;

pub async fn doh(req_wireformat: &[u8]) -> Result<Vec<u8>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/dns-message"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/dns-message"));
    
    let client = Client::new();
    let providers = [
        "https://1.1.1.1/dns-query",
        "https://8.8.8.8/dns-query",
    ];
    
    let mut last_error = None;
    
    for provider in providers.iter() {
        match client
            .post(*provider)
            .headers(headers.clone())
            .body(req_wireformat.to_vec())
            .send()
            .await
        {
            Ok(response) => {
                return response
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .context(format!("Failed to read response from {}", provider));
            }
            Err(e) => {
                last_error = Some(e);
                continue;
            }
        }
    }
    
    Err(last_error.unwrap()).context("All DoH providers failed")
}
