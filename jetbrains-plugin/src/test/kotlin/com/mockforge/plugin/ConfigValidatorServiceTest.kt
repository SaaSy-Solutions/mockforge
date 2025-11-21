package com.mockforge.plugin

import com.intellij.testFramework.fixtures.BasePlatformTestCase
import com.mockforge.plugin.services.ConfigValidatorService
import com.mockforge.plugin.services.SchemaType

/**
 * Tests for ConfigValidatorService
 */
class ConfigValidatorServiceTest : BasePlatformTestCase() {
    
    fun testDetectSchemaType() {
        val service = ConfigValidatorService.getInstance(myFixture.project)
        
        // Test main config file detection
        assertEquals(
            SchemaType.MOCKFORGE_CONFIG,
            service.detectSchemaType("mockforge.yaml", "/path/to/mockforge.yaml")
        )
        
        assertEquals(
            SchemaType.MOCKFORGE_CONFIG,
            service.detectSchemaType("mockforge.toml", "/path/to/mockforge.toml")
        )
        
        // Test blueprint file detection
        assertEquals(
            SchemaType.BLUEPRINT_CONFIG,
            service.detectSchemaType("blueprint.yaml", "/path/to/blueprint.yaml")
        )
        
        // Test reality config detection
        assertEquals(
            SchemaType.REALITY_CONFIG,
            service.detectSchemaType("reality.yaml", "/path/to/reality/reality.yaml")
        )
        
        // Test persona config detection
        assertEquals(
            SchemaType.PERSONA_CONFIG,
            service.detectSchemaType("persona.yaml", "/path/to/personas/persona.yaml")
        )
    }
    
    override fun getTestDataPath(): String {
        return "src/test/testData"
    }
}

