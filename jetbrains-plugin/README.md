# MockForge JetBrains Plugin

JetBrains IDE plugin for MockForge providing config validation, code actions, and inline preview.

## Features

- **Config Validation**: Real-time validation of `mockforge.yaml` and `mockforge.toml` files using JSON Schema
- **Autocomplete**: Intelligent autocomplete for configuration keys and values
- **Code Actions**: "Generate Mock Scenario" code action for OpenAPI specifications
- **Inline Preview**: Hover to preview mock responses for endpoints
- **Linting**: Linting for MockForge configuration files

## Implementation Status

This plugin is planned but not yet implemented. The implementation should follow the same patterns as the VS Code extension:

### Required Components

1. **Config Validator** (`ConfigValidator.kt`)
   - Load JSON Schema from `mockforge schema` command or schema file
   - Validate YAML/TOML config files against schema
   - Report validation errors as IDE inspections

2. **Language Server** (`MockForgeLanguageServer.kt`)
   - Register file type associations for MockForge config files
   - Provide hover documentation for config keys
   - Provide autocomplete suggestions
   - Show inline previews of mock responses

3. **Code Actions** (`GenerateMockScenarioAction.kt`)
   - Detect OpenAPI specifications
   - Generate MockForge scenario files from OpenAPI operations
   - Register as IntelliJ code action

4. **Inspections** (`MockForgeConfigInspection.kt`)
   - Real-time validation of config files
   - Highlight errors and warnings
   - Quick fixes for common issues

### Plugin Structure

```
jetbrains-plugin/
├── src/
│   ├── main/
│   │   ├── kotlin/
│   │   │   ├── com/
│   │   │   │   └── mockforge/
│   │   │   │       ├── plugin/
│   │   │   │       │   ├── MockForgePlugin.kt
│   │   │   │       │   ├── ConfigValidator.kt
│   │   │   │       │   ├── LanguageServer.kt
│   │   │   │       │   ├── GenerateMockScenarioAction.kt
│   │   │   │       │   └── inspections/
│   │   │   │       │       └── MockForgeConfigInspection.kt
│   │   │   │       └── ...
│   │   │   └── resources/
│   │   │       ├── META-INF/
│   │   │       │   └── plugin.xml
│   │   │       └── ...
│   │   └── test/
│   └── build.gradle.kts
├── README.md
└── build.gradle.kts
```

### Implementation Notes

1. **JSON Schema Validation**: Use a JSON Schema validator library (e.g., `com.github.everit-org.json-schema` or `com.networknt/json-schema-validator`)

2. **YAML/TOML Parsing**: Use existing IntelliJ YAML/TOML support or libraries like:
   - `org.yaml:snakeyaml` for YAML
   - `com.moandjiezana.toml:toml4j` for TOML

3. **File Type Registration**: Register MockForge config files in `plugin.xml`:
   ```xml
   <fileType name="MockForge Config" implementationClass="com.mockforge.plugin.MockForgeConfigFileType" extensions="mockforge.yaml;mockforge.yml;mockforge.toml" />
   ```

4. **Inspections**: Extend `LocalInspectionTool` to provide real-time validation

5. **Code Actions**: Extend `IntentionAction` or use `QuickFix` for code actions

### Dependencies

- IntelliJ Platform SDK
- Kotlin
- JSON Schema validator
- YAML/TOML parser

### Building

```bash
./gradlew buildPlugin
```

### Testing

```bash
./gradlew runIde
```

## Future Enhancements

- Integration with MockForge server for live preview
- Scenario execution from IDE
- Mock server control panel
- Request/response inspection
