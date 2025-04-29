use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use std::time::Duration;

pub async fn doh(req_wireformat: &[u8], endpoint: Option<&str>) -> Result<Vec<u8>> {
    // Prepare DNS-over-HTTPS headers
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/dns-message"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/dns-message"));
    headers.insert(USER_AGENT, HeaderValue::from_static("rust-doh-client/1.0"));

    // Create HTTP client (no timeout configuration available in WASM)
    let client = Client::new();

    // Use provided endpoint or default to Cloudflare
    let endpoint = endpoint.unwrap_or("https://cloudflare-dns.com/dns-query");

    // Send POST request to DoH endpoint
    let response = client
        .post(endpoint)
        .headers(headers)
        .body(req_wireformat.to_vec())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok(response.to_vec())
}
