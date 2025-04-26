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
    let body = req_wireformat.to_vec();
    
    // Try Cloudflare (1.1.1.1) first
    let response = client
        .post("https://1.1.1.1/dns-query")
        .headers(headers.clone())
        .body(body.clone())
        .send()
        .await;
    
    // If Cloudflare fails, try Google (8.8.8.8)
    let response = match response {
        Ok(resp) => Ok(resp),
        Err(_) => {
            client
                .post("https://8.8.8.8/dns-query")
                .headers(headers)
                .body(body)
                .send()
                .await
                .context("Both Cloudflare and Google DNS-over-HTTPS requests failed")
        }
    }?;
    
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}
