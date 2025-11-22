package com.mockforge.plugin.services

import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import com.networknt.schema.JsonSchema
import com.networknt.schema.JsonSchemaFactory
import com.networknt.schema.SpecVersion
import com.networknt.schema.ValidationMessage
import org.yaml.snakeyaml.Yaml
import java.io.File
import java.util.concurrent.ConcurrentHashMap

/**
 * Service for validating MockForge configuration files using JSON Schema
 *
 * This service:
 * - Loads JSON Schema from mockforge schema generate command or bundled schema files
 * - Caches schemas for performance
 * - Validates mockforge.yaml, mockforge.toml, blueprint.yaml files
 * - Reports validation errors for IDE inspections
 */
@Service(Service.Level.PROJECT)
class ConfigValidatorService(private val project: Project) {

    private val schemaCache = ConcurrentHashMap<String, JsonSchema>()
    private val schemaFactory = JsonSchemaFactory.getInstance(SpecVersion.VersionFlag.V7)

    companion object {
        fun getInstance(project: Project): ConfigValidatorService = project.service()
    }

    /**
     * Detect schema type based on file name and path
     */
    fun detectSchemaType(fileName: String, filePath: String): SchemaType? {
        val lowerFileName = fileName.lowercase()
        val lowerFilePath = filePath.lowercase()

        // Main config file
        if (lowerFileName == "mockforge.yaml" || lowerFileName == "mockforge.yml" ||
            lowerFileName == "mockforge.json" || lowerFileName == "mockforge.toml") {
            return SchemaType.MOCKFORGE_CONFIG
        }

        // Blueprint file
        if (lowerFileName == "blueprint.yaml" || lowerFileName == "blueprint.yml") {
            return SchemaType.BLUEPRINT_CONFIG
        }

        // Reality config (in reality/ directory or reality*.yaml)
        if (lowerFilePath.contains("/reality/") || lowerFileName.startsWith("reality")) {
            return SchemaType.REALITY_CONFIG
        }

        // Persona config (in personas/ directory)
        if (lowerFilePath.contains("/personas/")) {
            return SchemaType.PERSONA_CONFIG
        }

        // Try to detect from file pattern
        if (lowerFileName.endsWith(".mockforge.yaml") || lowerFileName.endsWith(".mockforge.yml")) {
            return SchemaType.MOCKFORGE_CONFIG
        }

        return null
    }

    /**
     * Get or load schema for a given schema type
     */
    fun getSchema(schemaType: SchemaType): JsonSchema? {
        return schemaCache.getOrPut(schemaType.name) {
            loadSchema(schemaType)
        }
    }

    /**
     * Load schema from file or generate from command
     */
    private fun loadSchema(schemaType: SchemaType): JsonSchema? {
        // Try to load from bundled schema file first
        val schemaResource = when (schemaType) {
            SchemaType.MOCKFORGE_CONFIG -> "/schemas/mockforge_config.schema.json"
            SchemaType.BLUEPRINT_CONFIG -> "/schemas/blueprint_config.schema.json"
            SchemaType.REALITY_CONFIG -> "/schemas/reality_config.schema.json"
            SchemaType.PERSONA_CONFIG -> "/schemas/persona_config.schema.json"
        }

        val schemaStream = javaClass.getResourceAsStream(schemaResource)
        if (schemaStream != null) {
            val schemaJson = schemaStream.bufferedReader().use { it.readText() }
            return schemaFactory.getSchema(schemaJson)
        }

        // Fallback: try to generate schema using mockforge CLI
        return generateSchemaFromCommand(schemaType)
    }

    /**
     * Generate schema using mockforge schema generate command
     */
    private fun generateSchemaFromCommand(schemaType: SchemaType): JsonSchema? {
        try {
            val process = ProcessBuilder("mockforge", "schema", "generate")
                .directory(File(project.basePath ?: "."))
                .start()

            val output = process.inputStream.bufferedReader().readText()
            process.waitFor()

            if (process.exitValue() == 0 && output.isNotBlank()) {
                return schemaFactory.getSchema(output)
            }
        } catch (e: Exception) {
            // Command not available or failed, return null
        }

        return null
    }

    /**
     * Validate a configuration file content
     */
    fun validate(content: String, schemaType: SchemaType): List<ValidationError> {
        val schema = getSchema(schemaType) ?: return emptyList()

        // Parse YAML/TOML to JSON
        val jsonContent = try {
            when {
                content.trimStart().startsWith("{") -> content // Already JSON
                else -> {
                    // Parse as YAML
                    val yaml = Yaml()
                    val parsed = yaml.load<Map<*, *>>(content)
                    com.google.gson.Gson().toJson(parsed)
                }
            }
        } catch (e: Exception) {
            return listOf(ValidationError(
                line = 0,
                column = 0,
                message = "Failed to parse configuration: ${e.message}",
                severity = ValidationSeverity.ERROR
            ))
        }

        // Validate against schema
        val objectMapper = ObjectMapper()
        val jsonNode: JsonNode = objectMapper.readTree(jsonContent)
        val validationMessages = schema.validate(jsonNode)

        return validationMessages.map { message ->
            ValidationError(
                line = extractLineNumber(message),
                column = extractColumnNumber(message),
                message = message.message,
                severity = ValidationSeverity.ERROR
            )
        }
    }

    private fun extractLineNumber(message: ValidationMessage): Int {
        // Extract line number from validation message path
        val path = message.path
        // Try to extract line number from path or message
        return 0 // Default, should be improved with actual parsing
    }

    private fun extractColumnNumber(message: ValidationMessage): Int {
        return 0 // Default, should be improved with actual parsing
    }

    /**
     * Clear schema cache (useful for testing or schema updates)
     */
    fun clearCache() {
        schemaCache.clear()
    }
}

/**
 * Schema type enumeration
 */
enum class SchemaType {
    MOCKFORGE_CONFIG,
    BLUEPRINT_CONFIG,
    REALITY_CONFIG,
    PERSONA_CONFIG
}

/**
 * Validation error result
 */
data class ValidationError(
    val line: Int,
    val column: Int,
    val message: String,
    val severity: ValidationSeverity
)

enum class ValidationSeverity {
    ERROR,
    WARNING,
    INFO
}
