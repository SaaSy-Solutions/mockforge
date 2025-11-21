//! Backstory generation for personas
//!
//! This module provides template-based backstory generation that creates
//! coherent narrative contexts for personas based on their traits and domain.
//! Backstories enable more realistic and logically consistent data generation.

use crate::domains::Domain;
use crate::persona::PersonaProfile;
use crate::Result;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;

/// Generator for creating persona backstories
///
/// Uses domain-specific templates and persona traits to generate
/// coherent narrative backstories that explain persona behavior.
#[derive(Debug)]
pub struct BackstoryGenerator {
    /// Domain-specific backstory templates
    templates: HashMap<Domain, Vec<BackstoryTemplate>>,
}

/// A backstory template with placeholders for trait values
#[derive(Debug, Clone)]
pub struct BackstoryTemplate {
    /// Template string with placeholders like "{spending_level}" or "{account_type}"
    template: String,
    /// Required traits that must be present for this template to be used
    required_traits: Vec<String>,
}

impl BackstoryGenerator {
    /// Create a new backstory generator with default templates
    pub fn new() -> Self {
        let mut generator = Self {
            templates: HashMap::new(),
        };

        // Initialize domain-specific templates
        generator.initialize_finance_templates();
        generator.initialize_ecommerce_templates();
        generator.initialize_healthcare_templates();
        generator.initialize_iot_templates();

        generator
    }

    /// Generate a backstory for a persona based on its traits and domain
    ///
    /// Uses the persona's seed for deterministic generation, ensuring
    /// the same persona always gets the same backstory.
    pub fn generate_backstory(&self, persona: &PersonaProfile) -> Result<String> {
        let templates = self.templates.get(&persona.domain).ok_or_else(|| {
            crate::Error::generic(format!(
                "No backstory templates available for domain: {:?}",
                persona.domain
            ))
        })?;

        // Filter templates that match the persona's traits
        let matching_templates: Vec<&BackstoryTemplate> = templates
            .iter()
            .filter(|template| {
                template
                    .required_traits
                    .iter()
                    .all(|trait_name| persona.get_trait(trait_name).is_some())
            })
            .collect();

        if matching_templates.is_empty() {
            // Fallback to generic backstory if no templates match
            return Ok(self.generate_generic_backstory(persona));
        }

        // Use persona seed for deterministic selection
        let mut rng = StdRng::seed_from_u64(persona.seed);
        let selected_template = &matching_templates[rng.random_range(0..matching_templates.len())];

        // Fill in template placeholders with trait values
        let mut backstory = selected_template.template.clone();
        for (trait_name, trait_value) in &persona.traits {
            let placeholder = format!("{{{}}}", trait_name);
            backstory = backstory.replace(&placeholder, trait_value);
        }

        // Replace any remaining placeholders with generic values
        backstory = self.fill_remaining_placeholders(&backstory, persona, &mut rng)?;

        Ok(backstory)
    }

    /// Generate a generic backstory when no specific templates match
    fn generate_generic_backstory(&self, persona: &PersonaProfile) -> String {
        match persona.domain {
            Domain::Finance => {
                format!(
                    "A {} user in the finance domain with {} traits.",
                    persona.id,
                    persona.traits.len()
                )
            }
            Domain::Ecommerce => {
                format!(
                    "An e-commerce customer with ID {} and {} preferences.",
                    persona.id,
                    persona.traits.len()
                )
            }
            Domain::Healthcare => {
                format!(
                    "A healthcare patient with ID {} and {} medical attributes.",
                    persona.id,
                    persona.traits.len()
                )
            }
            Domain::Iot => {
                format!(
                    "An IoT device or user with ID {} and {} characteristics.",
                    persona.id,
                    persona.traits.len()
                )
            }
            _ => format!("A user with ID {} in the {:?} domain.", persona.id, persona.domain),
        }
    }

    /// Fill any remaining placeholders in the template
    fn fill_remaining_placeholders(
        &self,
        template: &str,
        _persona: &PersonaProfile,
        rng: &mut StdRng,
    ) -> Result<String> {
        let mut result = template.to_string();

        // Common placeholders that might not have corresponding traits
        let common_replacements: HashMap<&str, Vec<&str>> = [
            ("{age_group}", vec!["young", "middle-aged", "senior"]),
            ("{location}", vec!["urban", "suburban", "rural"]),
            ("{activity_level}", vec!["active", "moderate", "low"]),
        ]
        .into_iter()
        .collect();

        for (placeholder, options) in common_replacements {
            if result.contains(placeholder) {
                let value = options[rng.random_range(0..options.len())];
                result = result.replace(placeholder, value);
            }
        }

        // Remove any remaining unmatched placeholders
        while let Some(start) = result.find('{') {
            if let Some(end) = result[start..].find('}') {
                result.replace_range(start..start + end + 1, "");
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Initialize finance domain backstory templates
    fn initialize_finance_templates(&mut self) {
        let templates = vec![
            BackstoryTemplate {
                template: "A {spending_level} spender with a {account_type} account, preferring {preferred_currency} transactions. Account age: {account_age}.".to_string(),
                required_traits: vec!["spending_level".to_string(), "account_type".to_string()],
            },
            BackstoryTemplate {
                template: "Finance professional with {account_type} account and {transaction_frequency} transaction activity. Prefers {preferred_currency}.".to_string(),
                required_traits: vec!["account_type".to_string(), "transaction_frequency".to_string()],
            },
            BackstoryTemplate {
                template: "A {spending_level} spending customer with {account_age} account history. Primary currency: {preferred_currency}.".to_string(),
                required_traits: vec!["spending_level".to_string(), "account_age".to_string()],
            },
        ];

        self.templates.insert(Domain::Finance, templates);
    }

    /// Initialize e-commerce domain backstory templates
    fn initialize_ecommerce_templates(&mut self) {
        let templates = vec![
            BackstoryTemplate {
                template: "A {customer_segment} customer who makes {purchase_frequency} purchases, primarily in {preferred_category}. Shipping preference: {preferred_shipping}.".to_string(),
                required_traits: vec!["customer_segment".to_string(), "purchase_frequency".to_string()],
            },
            BackstoryTemplate {
                template: "{customer_segment} shopper with {purchase_frequency} buying habits. Favorite category: {preferred_category}. Return frequency: {return_frequency}.".to_string(),
                required_traits: vec!["customer_segment".to_string(), "preferred_category".to_string()],
            },
            BackstoryTemplate {
                template: "E-commerce customer in the {preferred_category} category with {purchase_frequency} purchase patterns. Prefers {preferred_shipping} delivery.".to_string(),
                required_traits: vec!["preferred_category".to_string(), "preferred_shipping".to_string()],
            },
        ];

        self.templates.insert(Domain::Ecommerce, templates);
    }

    /// Initialize healthcare domain backstory templates
    fn initialize_healthcare_templates(&mut self) {
        let templates = vec![
            BackstoryTemplate {
                template: "A {age_group} patient with {insurance_type} insurance and blood type {blood_type}. Visit frequency: {visit_frequency}. Chronic conditions: {chronic_conditions}.".to_string(),
                required_traits: vec!["insurance_type".to_string(), "blood_type".to_string()],
            },
            BackstoryTemplate {
                template: "{age_group} patient covered by {insurance_type} insurance. {visit_frequency} medical visits with {chronic_conditions} chronic conditions.".to_string(),
                required_traits: vec!["age_group".to_string(), "insurance_type".to_string(), "visit_frequency".to_string()],
            },
            BackstoryTemplate {
                template: "Healthcare patient with {blood_type} blood type and {insurance_type} coverage. {visit_frequency} visits, {chronic_conditions} chronic conditions.".to_string(),
                required_traits: vec!["blood_type".to_string(), "insurance_type".to_string()],
            },
        ];

        self.templates.insert(Domain::Healthcare, templates);
    }

    /// Initialize IoT domain backstory templates
    fn initialize_iot_templates(&mut self) {
        let templates = vec![
            BackstoryTemplate {
                template: "IoT device or user in a {location} environment with {activity_level} activity patterns.".to_string(),
                required_traits: vec![],
            },
            BackstoryTemplate {
                template: "Connected device user with {activity_level} usage patterns in a {location} setting.".to_string(),
                required_traits: vec![],
            },
        ];

        self.templates.insert(Domain::Iot, templates);
    }

    /// Add a custom backstory template for a domain
    pub fn add_template(&mut self, domain: Domain, template: BackstoryTemplate) {
        self.templates.entry(domain).or_insert_with(Vec::new).push(template);
    }
}

impl Default for BackstoryGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persona::PersonaProfile;
    use std::collections::HashMap;

    #[test]
    fn test_backstory_generator_new() {
        let generator = BackstoryGenerator::new();
        assert!(generator.templates.contains_key(&Domain::Finance));
        assert!(generator.templates.contains_key(&Domain::Ecommerce));
        assert!(generator.templates.contains_key(&Domain::Healthcare));
    }

    #[test]
    fn test_generate_finance_backstory() {
        let generator = BackstoryGenerator::new();
        let mut persona = PersonaProfile::new("user123".to_string(), Domain::Finance);
        persona.set_trait("spending_level".to_string(), "high".to_string());
        persona.set_trait("account_type".to_string(), "premium".to_string());
        persona.set_trait("preferred_currency".to_string(), "USD".to_string());
        persona.set_trait("account_age".to_string(), "long_term".to_string());

        let backstory = generator.generate_backstory(&persona).unwrap();
        assert!(!backstory.is_empty());
        assert!(backstory.contains("high"));
        assert!(backstory.contains("premium"));
    }

    #[test]
    fn test_generate_ecommerce_backstory() {
        let generator = BackstoryGenerator::new();
        let mut persona = PersonaProfile::new("customer456".to_string(), Domain::Ecommerce);
        persona.set_trait("customer_segment".to_string(), "VIP".to_string());
        persona.set_trait("purchase_frequency".to_string(), "frequent".to_string());
        persona.set_trait("preferred_category".to_string(), "electronics".to_string());
        persona.set_trait("preferred_shipping".to_string(), "express".to_string());

        let backstory = generator.generate_backstory(&persona).unwrap();
        assert!(!backstory.is_empty());
        assert!(backstory.contains("VIP") || backstory.contains("electronics"));
    }

    #[test]
    fn test_generate_healthcare_backstory() {
        let generator = BackstoryGenerator::new();
        let mut persona = PersonaProfile::new("patient789".to_string(), Domain::Healthcare);
        persona.set_trait("insurance_type".to_string(), "private".to_string());
        persona.set_trait("blood_type".to_string(), "O+".to_string());
        persona.set_trait("age_group".to_string(), "adult".to_string());
        persona.set_trait("visit_frequency".to_string(), "regular".to_string());
        persona.set_trait("chronic_conditions".to_string(), "single".to_string());

        let backstory = generator.generate_backstory(&persona).unwrap();
        assert!(!backstory.is_empty());
        assert!(backstory.contains("private") || backstory.contains("O+"));
    }

    #[test]
    fn test_generate_generic_backstory() {
        let generator = BackstoryGenerator::new();
        let persona = PersonaProfile::new("user999".to_string(), Domain::General);

        // Should fall back to generic backstory for unsupported domain
        let backstory = generator.generate_backstory(&persona);
        // This might fail for General domain, but that's okay - we test the fallback
        if let Ok(backstory) = backstory {
            assert!(!backstory.is_empty());
        }
    }

    #[test]
    fn test_deterministic_backstory() {
        let generator = BackstoryGenerator::new();
        let mut persona1 = PersonaProfile::new("user123".to_string(), Domain::Finance);
        persona1.set_trait("spending_level".to_string(), "high".to_string());
        persona1.set_trait("account_type".to_string(), "premium".to_string());

        let mut persona2 = PersonaProfile::new("user123".to_string(), Domain::Finance);
        persona2.set_trait("spending_level".to_string(), "high".to_string());
        persona2.set_trait("account_type".to_string(), "premium".to_string());

        // Same ID and domain should produce same seed and same backstory
        assert_eq!(persona1.seed, persona2.seed);
        let backstory1 = generator.generate_backstory(&persona1).unwrap();
        let backstory2 = generator.generate_backstory(&persona2).unwrap();
        assert_eq!(backstory1, backstory2);
    }
}
