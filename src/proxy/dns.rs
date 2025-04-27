use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::Client;
use std::time::Duration;

pub async fn doh(req_wireformat: &[u8], max_retries: u8) -> Result<Vec<u8>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/dns-message"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/dns-message"));
    
    let client = Client::new();
    let body = req_wireformat.to_vec();
    let providers = [
        "https://8.8.8.8/dns-query",      // Google
        "https://dns.quad9.net/dns-query", // Quad9
        "https://1.1.1.1/dns-query",       // Cloudflare
    ];

    let mut last_error = None;
    
    for _ in 0..max_retries {
        for provider in &providers {
            match client
                .post(*provider)
                .headers(headers.clone())
                .body(body.clone())
                .send()
                .await
            {
                Ok(response) => {
                    let bytes = response.bytes().await?;
                    return Ok(bytes.to_vec());
                }
                Err(e) => {
                    last_error = Some(e.into()); // Konversi reqwest::Error ke anyhow::Error
                    continue;
                }
            }
        }
        
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Err(last_error.unwrap_or_else(|| {
        anyhow!("All DNS-over-HTTPS providers failed after {} retries", max_retries)
    }))
}
