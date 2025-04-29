use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::Client;
use std::time::Duration;

pub async fn doh(req_wireformat: &[u8]) -> Result<Vec<u8>> {
    // Siapkan header DNS-over-HTTPS
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/dns-message"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/dns-message"));

    // Buat client HTTP dengan timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?; // Tangani error pembuatan client

    // Kirim permintaan POST ke Cloudflare DoH endpoint
    let response = client
        .post("https://cloudflare-dns.com/dns-query") // Gunakan hostname, bukan IP
        .headers(headers)
        .body(req_wireformat.to_vec())
        .send()
        .await?
        .error_for_status()? // Tangani error HTTP
        .bytes()
        .await?; // Ambil isi respons sebagai bytes

    Ok(response.to_vec())
}
