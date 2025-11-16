//! HTTP middleware modules

pub mod deceptive_canary;
pub mod drift_tracking;
pub mod production_headers;
pub mod rate_limit;
pub mod response_buffer;
pub mod security;

pub use deceptive_canary::{deceptive_canary_middleware, DeceptiveCanaryState};
pub use drift_tracking::drift_tracking_middleware_with_extensions;
pub use production_headers::production_headers_middleware;
pub use rate_limit::{rate_limit_middleware, GlobalRateLimiter, RateLimitConfig};
pub use response_buffer::{buffer_response_middleware, get_buffered_response, BufferedResponse};
pub use security::security_middleware;
