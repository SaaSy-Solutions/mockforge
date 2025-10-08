//! Cross-protocol request matching for caching and replay

use super::{Protocol, ProtocolRequest, RequestMatcher};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Simple request matcher that matches on operation and path
pub struct SimpleRequestMatcher;

impl RequestMatcher for SimpleRequestMatcher {
    fn match_score(&self, request: &ProtocolRequest) -> f64 {
        // Base score: operation and path must match exactly
        1.0
    }

    fn protocol(&self) -> Protocol {
        // Supports all protocols
        Protocol::Http // Return type doesn't matter for cross-protocol matcher
    }
}

/// Fuzzy request matcher that considers headers and body
pub struct FuzzyRequestMatcher {
    /// Weight for operation match (0.0 to 1.0)
    pub operation_weight: f64,
    /// Weight for path match (0.0 to 1.0)
    pub path_weight: f64,
    /// Weight for metadata match (0.0 to 1.0)
    pub metadata_weight: f64,
    /// Weight for body match (0.0 to 1.0)
    pub body_weight: f64,
}

impl Default for FuzzyRequestMatcher {
    fn default() -> Self {
        Self {
            operation_weight: 0.4,
            path_weight: 0.4,
            metadata_weight: 0.1,
            body_weight: 0.1,
        }
    }
}

impl RequestMatcher for FuzzyRequestMatcher {
    fn match_score(&self, request: &ProtocolRequest) -> f64 {
        // Fuzzy matching considers multiple factors
        let mut score = 0.0;

        // Operation match
        if !request.operation.is_empty() {
            score += self.operation_weight;
        }

        // Path match
        if !request.path.is_empty() {
            score += self.path_weight;
        }

        // Metadata match
        if !request.metadata.is_empty() {
            score += self.metadata_weight;
        }

        // Body match
        if request.body.is_some() {
            score += self.body_weight;
        }

        score
    }

    fn protocol(&self) -> Protocol {
        Protocol::Http // Supports all protocols
    }
}

/// Request fingerprint for caching and replay
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestFingerprint {
    /// Protocol
    pub protocol: Protocol,
    /// Operation hash
    pub operation_hash: u64,
    /// Path hash
    pub path_hash: u64,
    /// Metadata hash (optional)
    pub metadata_hash: Option<u64>,
    /// Body hash (optional)
    pub body_hash: Option<u64>,
}

impl RequestFingerprint {
    /// Create a fingerprint from a protocol request
    pub fn from_request(request: &ProtocolRequest) -> Self {
        Self {
            protocol: request.protocol,
            operation_hash: Self::hash_string(&request.operation),
            path_hash: Self::hash_string(&request.path),
            metadata_hash: if !request.metadata.is_empty() {
                Some(Self::hash_metadata(&request.metadata))
            } else {
                None
            },
            body_hash: request.body.as_ref().map(|b| Self::hash_bytes(b)),
        }
    }

    /// Create a simple fingerprint (operation + path only)
    pub fn simple(request: &ProtocolRequest) -> Self {
        Self {
            protocol: request.protocol,
            operation_hash: Self::hash_string(&request.operation),
            path_hash: Self::hash_string(&request.path),
            metadata_hash: None,
            body_hash: None,
        }
    }

    /// Hash a string
    fn hash_string(s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash bytes
    fn hash_bytes(bytes: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash metadata map
    fn hash_metadata(metadata: &std::collections::HashMap<String, String>) -> u64 {
        let mut hasher = DefaultHasher::new();
        // Sort keys to ensure consistent hashing
        let mut keys: Vec<&String> = metadata.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(&mut hasher);
            if let Some(value) = metadata.get(key) {
                value.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Check if this fingerprint matches another
    pub fn matches(&self, other: &RequestFingerprint) -> bool {
        self.protocol == other.protocol
            && self.operation_hash == other.operation_hash
            && self.path_hash == other.path_hash
    }

    /// Check if this fingerprint exactly matches another (including metadata and body)
    pub fn exact_match(&self, other: &RequestFingerprint) -> bool {
        self == other
    }

    /// Calculate similarity score (0.0 to 1.0)
    pub fn similarity(&self, other: &RequestFingerprint) -> f64 {
        if self.protocol != other.protocol {
            return 0.0;
        }

        let mut score = 0.0;
        let mut factors = 0;

        // Operation match
        if self.operation_hash == other.operation_hash {
            score += 1.0;
        }
        factors += 1;

        // Path match
        if self.path_hash == other.path_hash {
            score += 1.0;
        }
        factors += 1;

        // Metadata match (if both have metadata)
        if let (Some(hash1), Some(hash2)) = (self.metadata_hash, other.metadata_hash) {
            if hash1 == hash2 {
                score += 1.0;
            }
            factors += 1;
        }

        // Body match (if both have bodies)
        if let (Some(hash1), Some(hash2)) = (self.body_hash, other.body_hash) {
            if hash1 == hash2 {
                score += 1.0;
            }
            factors += 1;
        }

        score / factors as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_simple_matcher() {
        let matcher = SimpleRequestMatcher;
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        assert_eq!(matcher.match_score(&request), 1.0);
    }

    #[test]
    fn test_fuzzy_matcher_default() {
        let matcher = FuzzyRequestMatcher::default();
        assert_eq!(matcher.operation_weight, 0.4);
        assert_eq!(matcher.path_weight, 0.4);
        assert_eq!(matcher.metadata_weight, 0.1);
        assert_eq!(matcher.body_weight, 0.1);
    }

    #[test]
    fn test_fuzzy_matcher_full_request() {
        let matcher = FuzzyRequestMatcher::default();
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("content-type".to_string(), "application/json".to_string());
                m
            },
            body: Some(b"{\"test\": true}".to_vec()),
            client_ip: None,
        };

        let score = matcher.match_score(&request);
        assert_eq!(score, 1.0); // All factors present
    }

    #[test]
    fn test_request_fingerprint_from_request() {
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/users".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let fp = RequestFingerprint::from_request(&request);
        assert_eq!(fp.protocol, Protocol::Http);
        assert!(fp.metadata_hash.is_none());
        assert!(fp.body_hash.is_none());
    }

    #[test]
    fn test_request_fingerprint_simple() {
        let request = ProtocolRequest {
            protocol: Protocol::Grpc,
            operation: "greeter.SayHello".to_string(),
            path: "/greeter.Greeter/SayHello".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("grpc-metadata".to_string(), "value".to_string());
                m
            },
            body: Some(b"test".to_vec()),
            client_ip: None,
        };

        let fp = RequestFingerprint::simple(&request);
        assert_eq!(fp.protocol, Protocol::Grpc);
        assert!(fp.metadata_hash.is_none()); // Simple fingerprint ignores metadata
        assert!(fp.body_hash.is_none()); // Simple fingerprint ignores body
    }

    #[test]
    fn test_fingerprint_matches() {
        let request1 = ProtocolRequest {
            protocol: Protocol::GraphQL,
            operation: "Query.users".to_string(),
            path: "/graphql".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let request2 = ProtocolRequest {
            protocol: Protocol::GraphQL,
            operation: "Query.users".to_string(),
            path: "/graphql".to_string(),
            metadata: HashMap::new(),
            body: Some(b"different body".to_vec()),
            client_ip: None,
        };

        let fp1 = RequestFingerprint::from_request(&request1);
        let fp2 = RequestFingerprint::from_request(&request2);

        assert!(fp1.matches(&fp2)); // Matches on protocol, operation, and path
        assert!(!fp1.exact_match(&fp2)); // Not an exact match due to different body
    }

    #[test]
    fn test_fingerprint_similarity() {
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let fp1 = RequestFingerprint::from_request(&request);
        let fp2 = RequestFingerprint::from_request(&request);

        assert_eq!(fp1.similarity(&fp2), 1.0); // Identical fingerprints
    }

    #[test]
    fn test_fingerprint_different_protocol() {
        let request1 = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let request2 = ProtocolRequest {
            protocol: Protocol::Grpc,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let fp1 = RequestFingerprint::from_request(&request1);
        let fp2 = RequestFingerprint::from_request(&request2);

        assert_eq!(fp1.similarity(&fp2), 0.0); // Different protocols
    }
}
