use std::collections::HashMap;

/// A binding between an exchange and a queue
#[derive(Debug, Clone)]
pub struct Binding {
    pub exchange: String,
    pub queue: String,
    pub routing_key: String,
    pub arguments: HashMap<String, String>,
}

impl Binding {
    pub fn new(exchange: String, queue: String, routing_key: String) -> Self {
        Self {
            exchange,
            queue,
            routing_key,
            arguments: HashMap::new(),
        }
    }

    /// Check if this binding matches the given routing key and headers
    pub fn matches(&self, routing_key: &str, _headers: &HashMap<String, String>) -> bool {
        // For now, simple routing key match
        self.routing_key == routing_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_new() {
        let binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "routing.key".to_string());

        assert_eq!(binding.exchange, "exchange1");
        assert_eq!(binding.queue, "queue1");
        assert_eq!(binding.routing_key, "routing.key");
        assert!(binding.arguments.is_empty());
    }

    #[test]
    fn test_binding_matches_exact() {
        let binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "user.created".to_string());

        let headers = HashMap::new();
        assert!(binding.matches("user.created", &headers));
    }

    #[test]
    fn test_binding_matches_no_match() {
        let binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "user.created".to_string());

        let headers = HashMap::new();
        assert!(!binding.matches("user.deleted", &headers));
        assert!(!binding.matches("order.created", &headers));
        assert!(!binding.matches("", &headers));
    }

    #[test]
    fn test_binding_with_arguments() {
        let mut binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "key".to_string());

        binding.arguments.insert("x-match".to_string(), "all".to_string());
        binding.arguments.insert("type".to_string(), "user".to_string());

        assert_eq!(binding.arguments.len(), 2);
        assert_eq!(binding.arguments.get("x-match"), Some(&"all".to_string()));
    }

    #[test]
    fn test_binding_clone() {
        let mut binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "key".to_string());
        binding.arguments.insert("test".to_string(), "value".to_string());

        let cloned = binding.clone();
        assert_eq!(binding.exchange, cloned.exchange);
        assert_eq!(binding.queue, cloned.queue);
        assert_eq!(binding.routing_key, cloned.routing_key);
        assert_eq!(binding.arguments.len(), cloned.arguments.len());
    }

    #[test]
    fn test_binding_debug() {
        let binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "key".to_string());

        let debug = format!("{:?}", binding);
        assert!(debug.contains("Binding"));
        assert!(debug.contains("exchange1"));
        assert!(debug.contains("queue1"));
        assert!(debug.contains("key"));
    }

    #[test]
    fn test_binding_matches_empty_routing_key() {
        let binding = Binding::new("exchange1".to_string(), "queue1".to_string(), "".to_string());

        let headers = HashMap::new();
        assert!(binding.matches("", &headers));
        assert!(!binding.matches("some.key", &headers));
    }

    #[test]
    fn test_binding_multiple_arguments() {
        let mut binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "key".to_string());

        binding.arguments.insert("arg1".to_string(), "val1".to_string());
        binding.arguments.insert("arg2".to_string(), "val2".to_string());
        binding.arguments.insert("arg3".to_string(), "val3".to_string());

        assert_eq!(binding.arguments.len(), 3);
        assert_eq!(binding.arguments.get("arg1"), Some(&"val1".to_string()));
        assert_eq!(binding.arguments.get("arg2"), Some(&"val2".to_string()));
        assert_eq!(binding.arguments.get("arg3"), Some(&"val3".to_string()));
    }

    #[test]
    fn test_binding_matches_with_headers_ignored() {
        let binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "test.key".to_string());

        let mut headers = HashMap::new();
        headers.insert("type".to_string(), "user".to_string());
        headers.insert("action".to_string(), "created".to_string());

        // Headers are currently ignored in the matches implementation
        assert!(binding.matches("test.key", &headers));
        assert!(!binding.matches("other.key", &headers));
    }

    #[test]
    fn test_binding_case_sensitive_routing_key() {
        let binding =
            Binding::new("exchange1".to_string(), "queue1".to_string(), "User.Created".to_string());

        let headers = HashMap::new();
        assert!(binding.matches("User.Created", &headers));
        assert!(!binding.matches("user.created", &headers));
        assert!(!binding.matches("USER.CREATED", &headers));
    }
}
