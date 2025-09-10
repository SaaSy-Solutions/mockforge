//! Data generator implementation

use crate::{
    faker::EnhancedFaker,
    schema::SchemaDefinition,
    DataConfig, GenerationResult,
};
use mockforge_core::Result;
use std::time::Instant;

/// Data generator for creating synthetic datasets
#[derive(Debug)]
pub struct DataGenerator {
    /// Schema definition
    schema: SchemaDefinition,
    /// Configuration
    config: DataConfig,
    /// Faker instance
    faker: EnhancedFaker,
    /// Seeded RNG if seed was provided
    seeded_rng: Option<rand::rngs::StdRng>,
}

impl DataGenerator {
    /// Create a new data generator
    pub fn new(schema: SchemaDefinition, config: DataConfig) -> Result<Self> {
        let faker = EnhancedFaker::new();
        let seeded_rng = if let Some(seed) = config.seed {
            use rand::SeedableRng;
            Some(rand::rngs::StdRng::seed_from_u64(seed))
        } else {
            None
        };

        Ok(Self {
            schema,
            config,
            faker,
            seeded_rng,
        })
    }

    /// Generate data according to the configuration
    pub async fn generate(&mut self) -> Result<GenerationResult> {
        let start_time = Instant::now();
        let mut data = Vec::with_capacity(self.config.rows);

        for _ in 0..self.config.rows {
            let row = self.schema.generate_row(&mut self.faker)?;
            data.push(row);
        }

        let generation_time = start_time.elapsed().as_millis();

        Ok(GenerationResult::new(data, generation_time))
    }

    /// Generate data with relationships resolved
    pub async fn generate_with_relationships(
        &mut self,
        related_schemas: &[SchemaDefinition],
    ) -> Result<GenerationResult> {
        let start_time = Instant::now();

        // Create a map of related schemas for lookup
        let schema_map: std::collections::HashMap<String, &SchemaDefinition> = related_schemas
            .iter()
            .map(|s| (s.name.clone(), s))
            .collect();

        let mut data = Vec::with_capacity(self.config.rows);

        for _ in 0..self.config.rows {
            let mut row = self.schema.generate_row(&mut self.faker)?;

            // Resolve relationships
            for (_rel_name, relationship) in &self.schema.relationships {
                if let Some(target_schema) = schema_map.get(&relationship.target_schema) {
                    // Generate a related row
                    let related_row = target_schema.generate_row(&mut self.faker)?;

                    // Extract the foreign key value
                    if let Some(related_obj) = related_row.as_object() {
                        if let Some(fk_value) = related_obj.get("id") {
                            // Insert the foreign key into the current row
                            if let Some(row_obj) = row.as_object_mut() {
                                row_obj.insert(relationship.foreign_key.clone(), fk_value.clone());
                            }
                        }
                    }
                }
            }

            data.push(row);
        }

        let generation_time = start_time.elapsed().as_millis();

        Ok(GenerationResult::new(data, generation_time))
    }

    /// Generate a single row
    pub fn generate_single(&mut self) -> Result<serde_json::Value> {
        self.schema.generate_row(&mut self.faker)
    }

    /// Get the schema being used
    pub fn schema(&self) -> &SchemaDefinition {
        &self.schema
    }

    /// Get the current configuration
    pub fn config(&self) -> &DataConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: DataConfig) -> Result<()> {
        self.config = config;

        // Re-seed if needed
        if let Some(seed) = self.config.seed {
            use rand::SeedableRng;
            self.seeded_rng = Some(rand::rngs::StdRng::seed_from_u64(seed));
        } else {
            self.seeded_rng = None;
        }

        Ok(())
    }
}

/// Batch data generator for generating multiple datasets
#[derive(Debug)]
pub struct BatchGenerator {
    /// Generators for different schemas
    generators: Vec<DataGenerator>,
    /// Global configuration
    config: DataConfig,
}

impl BatchGenerator {
    /// Create a new batch generator
    pub fn new(schemas: Vec<SchemaDefinition>, config: DataConfig) -> Result<Self> {
        let mut generators = Vec::new();

        for schema in schemas {
            let generator = DataGenerator::new(schema, config.clone())?;
            generators.push(generator);
        }

        Ok(Self {
            generators,
            config,
        })
    }

    /// Generate data for all schemas
    pub async fn generate_batch(&mut self) -> Result<Vec<GenerationResult>> {
        let mut results = Vec::new();

        for generator in &mut self.generators {
            let result = generator.generate().await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Generate data with cross-schema relationships
    pub async fn generate_with_relationships(&mut self) -> Result<Vec<GenerationResult>> {
        let mut results = Vec::new();
        let schemas: Vec<SchemaDefinition> = self.generators
            .iter()
            .map(|g| g.schema().clone())
            .collect();

        for generator in &mut self.generators {
            let result = generator.generate_with_relationships(&schemas).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get all schemas
    pub fn schemas(&self) -> Vec<&SchemaDefinition> {
        self.generators.iter().map(|g| g.schema()).collect()
    }
}

/// Utility functions for data generation
pub mod utils {
    use super::*;
    use mockforge_core::Result;

    /// Generate sample data from a simple schema definition
    pub async fn generate_sample_data(
        schema_name: &str,
        fields: Vec<(&str, &str)>,
        rows: usize,
    ) -> Result<GenerationResult> {
        let mut schema = SchemaDefinition::new(schema_name.to_string());

        for (field_name, field_type) in fields {
            let field = crate::schema::FieldDefinition::new(
                field_name.to_string(),
                field_type.to_string(),
            );
            schema = schema.with_field(field);
        }

        let config = DataConfig {
            rows,
            ..Default::default()
        };

        let mut generator = DataGenerator::new(schema, config)?;
        generator.generate().await
    }

    /// Generate user data
    pub async fn generate_users(count: usize) -> Result<GenerationResult> {
        let schema = crate::schema::templates::user_schema();
        let config = DataConfig {
            rows: count,
            ..Default::default()
        };

        let mut generator = DataGenerator::new(schema, config)?;
        generator.generate().await
    }

    /// Generate product data
    pub async fn generate_products(count: usize) -> Result<GenerationResult> {
        let schema = crate::schema::templates::product_schema();
        let config = DataConfig {
            rows: count,
            ..Default::default()
        };

        let mut generator = DataGenerator::new(schema, config)?;
        generator.generate().await
    }

    /// Generate orders with user relationships
    pub async fn generate_orders_with_users(order_count: usize, user_count: usize) -> Result<Vec<GenerationResult>> {
        let user_schema = crate::schema::templates::user_schema();
        let order_schema = crate::schema::templates::order_schema();

        let config = DataConfig {
            rows: order_count,
            ..Default::default()
        };

        let mut batch_generator = BatchGenerator::new(
            vec![user_schema, order_schema],
            config,
        )?;

        // Update the order generator to generate the right number of rows
        if let Some(order_generator) = batch_generator.generators.get_mut(1) {
            let order_config = DataConfig {
                rows: order_count,
                ..Default::default()
            };
            order_generator.update_config(order_config)?;
        }

        // Update the user generator to generate users
        if let Some(user_generator) = batch_generator.generators.get_mut(0) {
            let user_config = DataConfig {
                rows: user_count,
                ..Default::default()
            };
            user_generator.update_config(user_config)?;
        }

        batch_generator.generate_with_relationships().await
    }
}
