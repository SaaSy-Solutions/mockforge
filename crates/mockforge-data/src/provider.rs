use crate::faker::EnhancedFaker;
// NOTE: mockforge_core dependency removed to break circular dependency
// The provider registration functionality has been moved to a higher-level crate

// NOTE: FakerProvider trait and registration removed to break circular dependency
// This functionality should be implemented in a higher-level crate that depends on both
// mockforge-core and mockforge-data

#[allow(dead_code)]
struct DataFakerProvider(std::sync::Mutex<EnhancedFaker>);

impl DataFakerProvider {
    fn new() -> Self {
        Self(std::sync::Mutex::new(EnhancedFaker::new()))
    }
}

// NOTE: FakerProvider implementation removed - see comment above
/*
impl FakerProvider for DataFakerProvider {
    fn uuid(&self) -> String {
        self.0.lock().unwrap().uuid()
    }
    fn email(&self) -> String {
        self.0.lock().unwrap().email()
    }
    fn name(&self) -> String {
        self.0.lock().unwrap().name()
    }
    fn address(&self) -> String {
        self.0.lock().unwrap().address()
    }
    fn phone(&self) -> String {
        self.0.lock().unwrap().phone()
    }
    fn company(&self) -> String {
        self.0.lock().unwrap().company()
    }
    fn url(&self) -> String {
        self.0.lock().unwrap().url()
    }
    fn ip(&self) -> String {
        self.0.lock().unwrap().ip_address()
    }
    fn color(&self) -> String {
        self.0.lock().unwrap().color()
    }
    fn word(&self) -> String {
        self.0.lock().unwrap().word()
    }
    fn sentence(&self) -> String {
        self.0.lock().unwrap().sentence()
    }
    fn paragraph(&self) -> String {
        self.0.lock().unwrap().paragraph()
    }
}
*/

/// Register the mockforge-data backed faker provider with core templating.
/// NOTE: Disabled to break circular dependency
pub fn register_core_faker_provider() {
    // Disabled - functionality moved to higher-level crate
    // let provider: Arc<dyn FakerProvider + Send + Sync> = Arc::new(DataFakerProvider::new());
    // register_faker_provider(provider);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_faker_provider_new() {
        let _provider = DataFakerProvider::new();
        // Should create successfully
    }

    #[test]
    fn test_data_faker_provider_uuid() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let uuid = faker.uuid();

        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));
    }

    #[test]
    fn test_data_faker_provider_email() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let email = faker.email();

        assert!(!email.is_empty());
        assert!(email.contains('@'));
    }

    #[test]
    fn test_data_faker_provider_name() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let name = faker.name();

        assert!(!name.is_empty());
    }

    #[test]
    fn test_data_faker_provider_address() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let address = faker.address();

        assert!(!address.is_empty());
    }

    #[test]
    fn test_data_faker_provider_phone() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let phone = faker.phone();

        assert!(!phone.is_empty());
    }

    #[test]
    fn test_data_faker_provider_company() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let company = faker.company();

        assert!(!company.is_empty());
    }

    #[test]
    fn test_data_faker_provider_url() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let url = faker.url();

        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_data_faker_provider_ip() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let ip = faker.ip_address();

        assert!(!ip.is_empty());
        assert!(ip.contains('.'));
    }

    #[test]
    fn test_data_faker_provider_color() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let color = faker.color();

        let valid_colors = [
            "red", "blue", "green", "yellow", "purple", "orange", "pink", "brown", "black", "white",
        ];
        assert!(valid_colors.contains(&color.as_str()));
    }

    #[test]
    fn test_data_faker_provider_word() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let word = faker.word();

        assert!(!word.is_empty());
    }

    #[test]
    fn test_data_faker_provider_sentence() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let sentence = faker.sentence();

        assert!(!sentence.is_empty());
    }

    #[test]
    fn test_data_faker_provider_paragraph() {
        let provider = DataFakerProvider::new();
        let mut faker = provider.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let paragraph = faker.paragraph();

        assert!(!paragraph.is_empty());
    }

    #[test]
    fn test_register_core_faker_provider() {
        // Just test that registration doesn't panic
        register_core_faker_provider();
    }
}
