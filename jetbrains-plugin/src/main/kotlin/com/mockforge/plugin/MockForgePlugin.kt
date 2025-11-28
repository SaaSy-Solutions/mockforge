package com.mockforge.plugin

import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.StartupActivity
import com.mockforge.plugin.services.ConfigValidatorService
import com.mockforge.plugin.services.MockForgeClientService

/**
 * Main plugin class for MockForge JetBrains integration
 *
 * This plugin provides:
 * - Config validation for mockforge.yaml and mockforge.toml files
 * - Autocomplete for configuration keys and values
 * - Generate Mock Scenario code action for OpenAPI specifications
 * - Inline preview of mock responses when hovering over endpoint references
 * - Real-time linting for MockForge configuration files
 */
class MockForgePlugin : StartupActivity {

    override fun runActivity(project: Project) {
        // Initialize services when project opens
        // Services are lazy-loaded, so just accessing them initializes them
        ConfigValidatorService.getInstance(project)
        MockForgeClientService.getInstance(project)
    }
}
