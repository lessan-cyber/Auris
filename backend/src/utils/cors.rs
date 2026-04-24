use anyhow::{Context, Result};
use url::Url;

/// Parse CORS allowed origins from a comma-separated string
/// Validates that each origin is a valid URL with http/https scheme, has a host,
/// and has no path, query, or fragment components
pub fn parse_cors_origins(raw: &str) -> Result<Vec<String>> {
    let list: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    for origin in &list {
        // First parse as URL and validate structure
        let parsed_url = Url::parse(origin)
            .map_err(|e| anyhow::anyhow!("Invalid CORS origin URL '{}': {}", origin, e))?;

        // Validate scheme is http or https
        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Err(anyhow::anyhow!(
                "Invalid CORS origin '{}': scheme must be 'http' or 'https', got '{}'",
                origin,
                parsed_url.scheme()
            ));
        }

        // Validate that host is present
        if parsed_url.host().is_none() {
            return Err(anyhow::anyhow!(
                "Invalid CORS origin '{}': missing host",
                origin
            ));
        }

        // Validate no path component
        if !parsed_url.path().is_empty() && parsed_url.path() != "/" {
            return Err(anyhow::anyhow!(
                "Invalid CORS origin '{}': path component not allowed",
                origin
            ));
        }

        // Validate no query component
        if parsed_url.query().is_some() {
            return Err(anyhow::anyhow!(
                "Invalid CORS origin '{}': query component not allowed",
                origin
            ));
        }

        // Validate no fragment component
        if parsed_url.fragment().is_some() {
            return Err(anyhow::anyhow!(
                "Invalid CORS origin '{}': fragment component not allowed",
                origin
            ));
        }

        // Finally, ensure it can be converted to HeaderValue
        origin
            .parse::<axum::http::HeaderValue>()
            .map_err(|e| anyhow::anyhow!("Invalid CORS origin '{}': {}", origin, e))?;
    }

    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cors_origins() {
        // Test basic parsing
        let result = parse_cors_origins("http://localhost:5173,http://localhost:3000");
        assert!(result.is_ok());
        let origins = result.unwrap();
        assert_eq!(origins.len(), 2);
        assert_eq!(origins[0], "http://localhost:5173");
        assert_eq!(origins[1], "http://localhost:3000");

        // Test trimming
        let result = parse_cors_origins(" http://localhost:5173 , http://localhost:3000 ");
        assert!(result.is_ok());
        let origins = result.unwrap();
        assert_eq!(origins.len(), 2);
        assert_eq!(origins[0], "http://localhost:5173");

        // Test filtering empty strings
        let result = parse_cors_origins("http://localhost:5173,,http://localhost:3000");
        assert!(result.is_ok());
        let origins = result.unwrap();
        assert_eq!(origins.len(), 2);

        // Test single origin
        let result = parse_cors_origins("http://localhost:5173");
        assert!(result.is_ok());
        let origins = result.unwrap();
        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0], "http://localhost:5173");
    }

    #[test]
    fn test_parse_cors_origins_invalid() {
        // Test invalid origin with newlines (which are not allowed in HeaderValue)
        let result = parse_cors_origins("http://localhost:5173,http://\ninvalid.com");
        assert!(result.is_err());

        // Test missing scheme
        let result = parse_cors_origins("localhost:5173");
        assert!(result.is_err());

        // Test invalid scheme
        let result = parse_cors_origins("ftp://example.com");
        assert!(result.is_err());

        // Test missing host
        let result = parse_cors_origins("http://");
        assert!(result.is_err());

        // Test path component
        let result = parse_cors_origins("http://example.com/path");
        assert!(result.is_err());

        // Test query component
        let result = parse_cors_origins("http://example.com?param=value");
        assert!(result.is_err());

        // Test fragment component
        let result = parse_cors_origins("http://example.com#fragment");
        assert!(result.is_err());
    }
}
