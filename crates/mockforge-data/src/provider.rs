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
