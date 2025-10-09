use crate::faker::EnhancedFaker;
use mockforge_core::templating::{register_faker_provider, FakerProvider};
use std::sync::Arc;

struct DataFakerProvider(std::sync::Mutex<EnhancedFaker>);

impl DataFakerProvider {
    fn new() -> Self {
        Self(std::sync::Mutex::new(EnhancedFaker::new()))
    }
}

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

/// Register the mockforge-data backed faker provider with core templating.
pub fn register_core_faker_provider() {
    let provider: Arc<dyn FakerProvider + Send + Sync> = Arc::new(DataFakerProvider::new());
    register_faker_provider(provider);
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
        let uuid = provider.uuid();

        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));
    }

    #[test]
    fn test_data_faker_provider_email() {
        let provider = DataFakerProvider::new();
        let email = provider.email();

        assert!(!email.is_empty());
        assert!(email.contains('@'));
    }

    #[test]
    fn test_data_faker_provider_name() {
        let provider = DataFakerProvider::new();
        let name = provider.name();

        assert!(!name.is_empty());
    }

    #[test]
    fn test_data_faker_provider_address() {
        let provider = DataFakerProvider::new();
        let address = provider.address();

        assert!(!address.is_empty());
    }

    #[test]
    fn test_data_faker_provider_phone() {
        let provider = DataFakerProvider::new();
        let phone = provider.phone();

        assert!(!phone.is_empty());
    }

    #[test]
    fn test_data_faker_provider_company() {
        let provider = DataFakerProvider::new();
        let company = provider.company();

        assert!(!company.is_empty());
    }

    #[test]
    fn test_data_faker_provider_url() {
        let provider = DataFakerProvider::new();
        let url = provider.url();

        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_data_faker_provider_ip() {
        let provider = DataFakerProvider::new();
        let ip = provider.ip();

        assert!(!ip.is_empty());
        assert!(ip.contains('.'));
    }

    #[test]
    fn test_data_faker_provider_color() {
        let provider = DataFakerProvider::new();
        let color = provider.color();

        let valid_colors = ["red", "blue", "green", "yellow", "purple", "orange", "pink", "brown", "black", "white"];
        assert!(valid_colors.contains(&color.as_str()));
    }

    #[test]
    fn test_data_faker_provider_word() {
        let provider = DataFakerProvider::new();
        let word = provider.word();

        assert!(!word.is_empty());
    }

    #[test]
    fn test_data_faker_provider_sentence() {
        let provider = DataFakerProvider::new();
        let sentence = provider.sentence();

        assert!(!sentence.is_empty());
    }

    #[test]
    fn test_data_faker_provider_paragraph() {
        let provider = DataFakerProvider::new();
        let paragraph = provider.paragraph();

        assert!(!paragraph.is_empty());
    }

    #[test]
    fn test_register_core_faker_provider() {
        // Just test that registration doesn't panic
        register_core_faker_provider();
    }

    #[test]
    fn test_provider_trait_usage() {
        let provider: Arc<dyn FakerProvider + Send + Sync> = Arc::new(DataFakerProvider::new());

        let uuid = provider.uuid();
        assert_eq!(uuid.len(), 36);
    }
}
