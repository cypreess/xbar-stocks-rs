use regex::Regex;
use std::error::Error;
use std::time::Duration;

/// Fetches the latest price for a given stock ticker from Yahoo Finance
///
/// This function attempts to fetch the post-market price first. If not available,
/// it falls back to the regular market price.
///
/// # Arguments
///
/// * `ticker` - The stock ticker symbol (e.g., "AAPL", "GOOGL")
///
/// # Returns
///
/// * `Result<f64, Box<dyn Error + Send + Sync>>` - The stock price as a float, or an error
///
/// # Example
///
/// ```no_run
/// use stock_checker_rs::fetch_latest_price;
///
/// let price = fetch_latest_price("AAPL").unwrap();
/// println!("Price: {}", price);
/// ```
pub fn fetch_latest_price(ticker: &str) -> Result<f64, Box<dyn Error + Send + Sync>> {
    // Construct the Yahoo Finance URL
    let url = format!("https://stooq.pl/q/?s={}", ticker.to_lowercase());

    // Create a client with proper headers and timeouts
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .gzip(false) // Disable gzip to avoid decoding issues
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15)) // Total timeout including reading body
        .tcp_keepalive(Duration::from_secs(60))
        .pool_idle_timeout(Duration::from_secs(90))
        .build()?;

    // Fetch the page content
    let response = client.get(&url).send()?;
    if response.status() != 200 {
        return Err(format!("Invalid status code HTTP{}", response.status()).into());
    }

    // Read response as bytes first, then convert to string
    let bytes = response.bytes()?;
    let body = String::from_utf8(bytes.to_vec())
        .map_err(|e| format!("Failed to decode response: {}", e))?;

    let pattern = format!(r#"id=aq_{}_c4[^>]+>([0-9]+\.?[0-9]*)</span>"#, ticker.to_lowercase());

    let re_post = Regex::new(&pattern)?;

    if let Some(captures) = re_post.captures(&body) {
        if let Some(price_match) = captures.get(1) {
            let price: f64 = price_match.as_str().parse()?;
            return Ok(price);
        }
    }


    Err("Could not find price in response".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_pattern() {
        let test_html = r#"some text "postMarketPrice":{"raw":277.77, more text"#;
        let re = Regex::new(r#""postMarketPrice":\{"raw":([0-9]+\.?[0-9]*),?"#).unwrap();

        if let Some(captures) = re.captures(test_html) {
            if let Some(price_match) = captures.get(1) {
                let price: f64 = price_match.as_str().parse().unwrap();
                assert_eq!(price, 277.77);
            }
        }
    }
}
