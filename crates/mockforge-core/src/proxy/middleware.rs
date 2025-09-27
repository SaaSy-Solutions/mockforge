//! Proxy middleware for request/response processing

use crate::Result;
use std::collections::HashMap;

/// Middleware trait for processing proxy requests
pub trait ProxyMiddleware {
    /// Process a request before it's sent to the target
    fn process_request(
        &self,
        method: &str,
        url: &str,
        headers: &mut HashMap<String, String>,
        body: &mut Option<Vec<u8>>,
    ) -> Result<()>;

    /// Process a response before it's returned to the client
    fn process_response(
        &self,
        status_code: u16,
        headers: &mut HashMap<String, String>,
        body: &mut Option<Vec<u8>>,
    ) -> Result<()>;
}

/// Logging middleware that logs proxy requests and responses
pub struct LoggingMiddleware;

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggingMiddleware {
    /// Create a new logging middleware
    pub fn new() -> Self {
        Self
    }
}

impl ProxyMiddleware for LoggingMiddleware {
    fn process_request(
        &self,
        method: &str,
        url: &str,
        _headers: &mut HashMap<String, String>,
        _body: &mut Option<Vec<u8>>,
    ) -> Result<()> {
        tracing::info!("Proxy request: {} {}", method, url);
        Ok(())
    }

    fn process_response(
        &self,
        status_code: u16,
        _headers: &mut HashMap<String, String>,
        _body: &mut Option<Vec<u8>>,
    ) -> Result<()> {
        tracing::info!("Proxy response: {}", status_code);
        Ok(())
    }
}

/// Header modification middleware
pub struct HeaderMiddleware {
    /// Headers to add to requests
    request_headers: HashMap<String, String>,
    /// Headers to add to responses
    response_headers: HashMap<String, String>,
}

impl Default for HeaderMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl HeaderMiddleware {
    /// Create a new header middleware
    pub fn new() -> Self {
        Self {
            request_headers: HashMap::new(),
            response_headers: HashMap::new(),
        }
    }

    /// Add a header to outgoing requests
    pub fn add_request_header(mut self, key: String, value: String) -> Self {
        self.request_headers.insert(key, value);
        self
    }

    /// Add a header to outgoing responses
    pub fn add_response_header(mut self, key: String, value: String) -> Self {
        self.response_headers.insert(key, value);
        self
    }
}

impl ProxyMiddleware for HeaderMiddleware {
    fn process_request(
        &self,
        _method: &str,
        _url: &str,
        headers: &mut HashMap<String, String>,
        _body: &mut Option<Vec<u8>>,
    ) -> Result<()> {
        for (key, value) in &self.request_headers {
            headers.insert(key.clone(), value.clone());
        }
        Ok(())
    }

    fn process_response(
        &self,
        _status_code: u16,
        headers: &mut HashMap<String, String>,
        _body: &mut Option<Vec<u8>>,
    ) -> Result<()> {
        for (key, value) in &self.response_headers {
            headers.insert(key.clone(), value.clone());
        }
        Ok(())
    }
}
