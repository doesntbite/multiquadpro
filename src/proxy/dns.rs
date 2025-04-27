use futures_util::future::select_ok;
use reqwest::Client;

pub async fn doh(req_wireformat: &[u8]) -> Result<Vec<u8>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/dns-message"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/dns-message"));

    let providers = [
        "https://1.1.1.1/dns-query",
        "https://8.8.8.8/dns-query",
        "https://dns.google/dns-query",
    ];

    let client = Client::new();
    let requests = providers.iter().map(|&provider| {
        client
            .post(provider)
            .headers(headers.clone())
            .body(req_wireformat.to_vec())
            .send()
    });

    let result = select_ok(requests).await?;
    let bytes = result.0.bytes().await?;
    Ok(bytes.to_vec())
}
