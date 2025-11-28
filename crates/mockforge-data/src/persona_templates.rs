//! Domain-specific persona templates for generating realistic trait patterns
//!
//! This module provides templates for different business domains that define
//! how persona traits should be generated based on a seed. Each template
//! creates believable trait combinations that reflect real-world patterns.

use crate::domains::Domain;
use crate::persona::PersonaProfile;
use crate::persona_backstory::BackstoryGenerator;
use crate::Result;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;

/// Trait for persona templates that generate traits based on a seed
pub trait PersonaTemplate: Send + Sync {
    /// Generate traits for a persona based on its seed
    fn generate_traits(&self, seed: u64) -> HashMap<String, String>;

    /// Get the domain this template applies to
    fn domain(&self) -> Domain;

    /// Generate a backstory for a persona based on its traits
    ///
    /// This is an optional method that can be overridden by implementations
    /// to provide domain-specific backstory generation. The default implementation
    /// uses the BackstoryGenerator.
    fn generate_backstory(&self, persona: &PersonaProfile) -> Result<Option<String>> {
        let backstory_generator = BackstoryGenerator::new();
        match backstory_generator.generate_backstory(persona) {
            Ok(backstory) => Ok(Some(backstory)),
            Err(_) => Ok(None), // Return None if backstory generation fails
        }
    }
}

/// Finance persona template
///
/// Generates traits for financial personas including account types,
/// spending patterns, transaction frequency, and currency preferences.
pub struct FinancePersonaTemplate;

impl FinancePersonaTemplate {
    /// Create a new finance persona template
    pub fn new() -> Self {
        Self
    }
}

impl PersonaTemplate for FinancePersonaTemplate {
    fn domain(&self) -> Domain {
        Domain::Finance
    }

    fn generate_traits(&self, seed: u64) -> HashMap<String, String> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut traits = HashMap::new();

        // Account type: checking, savings, premium, business
        let account_types = ["checking", "savings", "premium", "business"];
        let account_type_idx = rng.random_range(0..account_types.len());
        traits.insert("account_type".to_string(), account_types[account_type_idx].to_string());

        // Spending level: conservative, moderate, high
        let spending_levels = ["conservative", "moderate", "high"];
        let spending_idx = rng.random_range(0..spending_levels.len());
        traits.insert("spending_level".to_string(), spending_levels[spending_idx].to_string());

        // Transaction frequency: low, medium, high
        let frequencies = ["low", "medium", "high"];
        let freq_idx = rng.random_range(0..frequencies.len());
        traits.insert("transaction_frequency".to_string(), frequencies[freq_idx].to_string());

        // Preferred currency: USD, EUR, GBP, JPY, CNY
        let currencies = ["USD", "EUR", "GBP", "JPY", "CNY"];
        let currency_idx = rng.random_range(0..currencies.len());
        traits.insert("preferred_currency".to_string(), currencies[currency_idx].to_string());

        // Account age: new, established, long_term
        let account_ages = ["new", "established", "long_term"];
        let age_idx = rng.random_range(0..account_ages.len());
        traits.insert("account_age".to_string(), account_ages[age_idx].to_string());

        traits
    }
}

impl Default for FinancePersonaTemplate {
    fn default() -> Self {
        Self::new()
    }
}

/// E-commerce persona template
///
/// Generates traits for e-commerce personas including customer segments,
/// purchase history patterns, product preferences, and shipping preferences.
pub struct EcommercePersonaTemplate;

impl EcommercePersonaTemplate {
    /// Create a new e-commerce persona template
    pub fn new() -> Self {
        Self
    }
}

impl PersonaTemplate for EcommercePersonaTemplate {
    fn domain(&self) -> Domain {
        Domain::Ecommerce
    }

    fn generate_traits(&self, seed: u64) -> HashMap<String, String> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut traits = HashMap::new();

        // Customer segment: VIP, regular, new
        let segments = ["VIP", "regular", "new"];
        let segment_idx = rng.random_range(0..segments.len());
        traits.insert("customer_segment".to_string(), segments[segment_idx].to_string());

        // Purchase frequency: occasional, regular, frequent
        let frequencies = ["occasional", "regular", "frequent"];
        let freq_idx = rng.random_range(0..frequencies.len());
        traits.insert("purchase_frequency".to_string(), frequencies[freq_idx].to_string());

        // Product category preference
        let categories = ["electronics", "clothing", "books", "home", "sports"];
        let cat_idx = rng.random_range(0..categories.len());
        traits.insert("preferred_category".to_string(), categories[cat_idx].to_string());

        // Shipping preference: standard, express, overnight
        let shipping = ["standard", "express", "overnight"];
        let ship_idx = rng.random_range(0..shipping.len());
        traits.insert("preferred_shipping".to_string(), shipping[ship_idx].to_string());

        // Return frequency: low, medium, high
        let return_freqs = ["low", "medium", "high"];
        let ret_idx = rng.random_range(0..return_freqs.len());
        traits.insert("return_frequency".to_string(), return_freqs[ret_idx].to_string());

        traits
    }
}

impl Default for EcommercePersonaTemplate {
    fn default() -> Self {
        Self::new()
    }
}

/// Healthcare persona template
///
/// Generates traits for healthcare personas including patient demographics,
/// medical history patterns, insurance types, and condition patterns.
pub struct HealthcarePersonaTemplate;

impl HealthcarePersonaTemplate {
    /// Create a new healthcare persona template
    pub fn new() -> Self {
        Self
    }
}

impl PersonaTemplate for HealthcarePersonaTemplate {
    fn domain(&self) -> Domain {
        Domain::Healthcare
    }

    fn generate_traits(&self, seed: u64) -> HashMap<String, String> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut traits = HashMap::new();

        // Insurance type: private, medicare, medicaid, uninsured
        let insurance_types = ["private", "medicare", "medicaid", "uninsured"];
        let ins_idx = rng.random_range(0..insurance_types.len());
        traits.insert("insurance_type".to_string(), insurance_types[ins_idx].to_string());

        // Blood type: A+, A-, B+, B-, AB+, AB-, O+, O-
        let blood_types = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
        let blood_idx = rng.random_range(0..blood_types.len());
        traits.insert("blood_type".to_string(), blood_types[blood_idx].to_string());

        // Age group: pediatric, adult, senior
        let age_groups = ["pediatric", "adult", "senior"];
        let age_idx = rng.random_range(0..age_groups.len());
        traits.insert("age_group".to_string(), age_groups[age_idx].to_string());

        // Visit frequency: rare, occasional, regular, frequent
        let visit_freqs = ["rare", "occasional", "regular", "frequent"];
        let visit_idx = rng.random_range(0..visit_freqs.len());
        traits.insert("visit_frequency".to_string(), visit_freqs[visit_idx].to_string());

        // Chronic conditions: none, single, multiple
        let conditions = ["none", "single", "multiple"];
        let cond_idx = rng.random_range(0..conditions.len());
        traits.insert("chronic_conditions".to_string(), conditions[cond_idx].to_string());

        traits
    }
}

impl Default for HealthcarePersonaTemplate {
    fn default() -> Self {
        Self::new()
    }
}

/// Template registry for managing domain-specific templates
pub struct PersonaTemplateRegistry {
    templates: HashMap<Domain, Box<dyn PersonaTemplate + Send + Sync>>,
}

impl PersonaTemplateRegistry {
    /// Create a new template registry with default templates
    pub fn new() -> Self {
        let mut registry = Self {
            templates: HashMap::new(),
        };

        // Register default templates
        registry.register_template(Domain::Finance, Box::new(FinancePersonaTemplate::new()));
        registry.register_template(Domain::Ecommerce, Box::new(EcommercePersonaTemplate::new()));
        registry.register_template(Domain::Healthcare, Box::new(HealthcarePersonaTemplate::new()));

        registry
    }

    /// Register a template for a domain
    pub fn register_template(
        &mut self,
        domain: Domain,
        template: Box<dyn PersonaTemplate + Send + Sync>,
    ) {
        self.templates.insert(domain, template);
    }

    /// Get a template for a domain
    pub fn get_template(&self, domain: Domain) -> Option<&(dyn PersonaTemplate + Send + Sync)> {
        self.templates.get(&domain).map(|t| t.as_ref())
    }

    /// Generate traits for a persona using the appropriate template
    pub fn generate_traits_for_persona(&self, persona: &PersonaProfile) -> HashMap<String, String> {
        if let Some(template) = self.get_template(persona.domain) {
            template.generate_traits(persona.seed)
        } else {
            HashMap::new()
        }
    }

    /// Apply template traits to a persona
    ///
    /// Generates traits using the template and adds them to the persona.
    pub fn apply_template_to_persona(&self, persona: &mut PersonaProfile) -> Result<()> {
        let traits = self.generate_traits_for_persona(persona);
        for (key, value) in traits {
            persona.set_trait(key, value);
        }
        Ok(())
    }

    /// Apply template traits and optionally generate a backstory
    ///
    /// Generates traits using the template, adds them to the persona, and
    /// optionally generates a backstory if `generate_backstory` is true.
    pub fn apply_template_to_persona_with_backstory(
        &self,
        persona: &mut PersonaProfile,
        generate_backstory: bool,
    ) -> Result<()> {
        // First apply traits
        self.apply_template_to_persona(persona)?;

        // Then generate backstory if requested
        if generate_backstory {
            if let Some(template) = self.get_template(persona.domain) {
                if let Ok(Some(backstory)) = template.generate_backstory(persona) {
                    persona.set_backstory(backstory);
                }
            }
        }

        Ok(())
    }
}

impl Default for PersonaTemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finance_template_generate_traits() {
        let template = FinancePersonaTemplate::new();
        let traits = template.generate_traits(42);

        assert!(traits.contains_key("account_type"));
        assert!(traits.contains_key("spending_level"));
        assert!(traits.contains_key("transaction_frequency"));
        assert!(traits.contains_key("preferred_currency"));
    }

    #[test]
    fn test_finance_template_deterministic() {
        let template = FinancePersonaTemplate::new();
        let traits1 = template.generate_traits(42);
        let traits2 = template.generate_traits(42);

        // Same seed should produce same traits
        assert_eq!(traits1, traits2);
    }

    #[test]
    fn test_ecommerce_template_generate_traits() {
        let template = EcommercePersonaTemplate::new();
        let traits = template.generate_traits(42);

        assert!(traits.contains_key("customer_segment"));
        assert!(traits.contains_key("purchase_frequency"));
        assert!(traits.contains_key("preferred_category"));
        assert!(traits.contains_key("preferred_shipping"));
    }

    #[test]
    fn test_healthcare_template_generate_traits() {
        let template = HealthcarePersonaTemplate::new();
        let traits = template.generate_traits(42);

        assert!(traits.contains_key("insurance_type"));
        assert!(traits.contains_key("blood_type"));
        assert!(traits.contains_key("age_group"));
        assert!(traits.contains_key("visit_frequency"));
    }

    #[test]
    fn test_template_registry() {
        let registry = PersonaTemplateRegistry::new();

        assert!(registry.get_template(Domain::Finance).is_some());
        assert!(registry.get_template(Domain::Ecommerce).is_some());
        assert!(registry.get_template(Domain::Healthcare).is_some());
        assert!(registry.get_template(Domain::Iot).is_none());
    }

    #[test]
    fn test_template_registry_apply_to_persona() {
        let registry = PersonaTemplateRegistry::new();
        let mut persona = PersonaProfile::new("user123".to_string(), Domain::Finance);

        registry.apply_template_to_persona(&mut persona).unwrap();

        assert!(persona.get_trait("account_type").is_some());
        assert!(persona.get_trait("spending_level").is_some());
    }
}
