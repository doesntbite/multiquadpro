use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::Client;

pub async fn doh(req_wireformat: &[u8], use_cloudflare: bool) -> Result<Vec<u8>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/dns-message"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/dns-message"));
    
    // Pilih URL berdasarkan parameter `use_cloudflare`
    let url = if use_cloudflare {
        "https://1.1.1.1/dns-query" // Cloudflare DNS-over-HTTPS
    } else {
        "https://8.8.8.8/dns-query" // Google DNS-over-HTTPS
    };

    let client = Client::new();
    let response = client
        .post(url)
        .headers(headers)
        .body(req_wireformat.to_vec())
        .send()
        .await?
        .bytes()
        .await?;

    Ok(response.to_vec())
}
