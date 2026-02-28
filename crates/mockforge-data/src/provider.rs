/// Register the mockforge-data backed faker provider with core templating.
///
/// This function is intentionally a no-op. The original implementation was disabled to break
/// a circular dependency between `mockforge-core` and `mockforge-data`. Faker functionality
/// is instead provided directly by `mockforge-core`'s built-in template expansion, which
/// handles `{{faker.*}}` placeholders without requiring a separate provider registration.
///
/// Callers (mockforge-http, mockforge-ws, mockforge-grpc) invoke this behind
/// `#[cfg(feature = "data-faker")]` for forward compatibility if provider registration
/// is re-enabled in the future.
pub fn register_core_faker_provider() {
    // No-op: faker functionality is provided by mockforge-core's template expansion
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faker::EnhancedFaker;

    #[test]
    fn test_register_core_faker_provider() {
        // Verify registration doesn't panic
        register_core_faker_provider();
    }

    #[test]
    fn test_enhanced_faker_uuid() {
        let mut faker = EnhancedFaker::new();
        let uuid = faker.uuid();
        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));
    }

    #[test]
    fn test_enhanced_faker_email() {
        let mut faker = EnhancedFaker::new();
        let email = faker.email();
        assert!(!email.is_empty());
        assert!(email.contains('@'));
    }

    #[test]
    fn test_enhanced_faker_name() {
        let mut faker = EnhancedFaker::new();
        let name = faker.name();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_enhanced_faker_address() {
        let mut faker = EnhancedFaker::new();
        let address = faker.address();
        assert!(!address.is_empty());
    }

    #[test]
    fn test_enhanced_faker_phone() {
        let mut faker = EnhancedFaker::new();
        let phone = faker.phone();
        assert!(!phone.is_empty());
    }

    #[test]
    fn test_enhanced_faker_company() {
        let mut faker = EnhancedFaker::new();
        let company = faker.company();
        assert!(!company.is_empty());
    }

    #[test]
    fn test_enhanced_faker_url() {
        let mut faker = EnhancedFaker::new();
        let url = faker.url();
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_enhanced_faker_ip() {
        let mut faker = EnhancedFaker::new();
        let ip = faker.ip_address();
        assert!(!ip.is_empty());
        assert!(ip.contains('.'));
    }

    #[test]
    fn test_enhanced_faker_color() {
        let mut faker = EnhancedFaker::new();
        let color = faker.color();
        let valid_colors = [
            "red", "blue", "green", "yellow", "purple", "orange", "pink", "brown", "black", "white",
        ];
        assert!(valid_colors.contains(&color.as_str()));
    }

    #[test]
    fn test_enhanced_faker_word() {
        let mut faker = EnhancedFaker::new();
        let word = faker.word();
        assert!(!word.is_empty());
    }

    #[test]
    fn test_enhanced_faker_sentence() {
        let mut faker = EnhancedFaker::new();
        let sentence = faker.sentence();
        assert!(!sentence.is_empty());
    }

    #[test]
    fn test_enhanced_faker_paragraph() {
        let mut faker = EnhancedFaker::new();
        let paragraph = faker.paragraph();
        assert!(!paragraph.is_empty());
    }
}
