# MockForge JetBrains Plugin

JetBrains IDE plugin for MockForge providing config validation, code actions, and inline preview.

## Features

- **Config Validation**: Real-time validation of `mockforge.yaml` and `mockforge.toml` files using JSON Schema
- **Autocomplete**: Intelligent autocomplete for configuration keys and values
- **Code Actions**: "Generate Mock Scenario" code action for OpenAPI specifications
- **Inline Preview**: Hover to preview mock responses for endpoints
- **Linting**: Linting for MockForge configuration files

## Installation

### From JetBrains Marketplace

1. Open your JetBrains IDE (IntelliJ IDEA, WebStorm, PyCharm, etc.)
2. Go to Settings/Preferences → Plugins
3. Search for "MockForge"
4. Click Install
5. Restart your IDE

### From Local Build

1. Build the plugin:
   ```bash
   ./gradlew buildPlugin
   ```

2. Install the plugin:
   - Go to Settings/Preferences → Plugins
   - Click the gear icon → Install Plugin from Disk
   - Select the generated `.zip` file from `build/distributions/`

## Usage

### Config Validation

Open any `mockforge.yaml` or `mockforge.toml` file. The plugin will automatically:
- Validate the configuration against JSON Schema
- Show errors and warnings inline
- Provide autocomplete suggestions

### Generate Mock Scenario

1. Open an OpenAPI specification file (`.yaml`, `.yml`, or `.json`)
2. Right-click in the editor
3. Select "Generate MockForge Scenario"
4. Choose which operations to include
5. Enter a scenario name
6. The scenario file will be generated and opened

### Inline Preview

Hover over API endpoint references in your code (TypeScript, JavaScript, Python, etc.) to see:
- Mock response preview
- Response headers and body
- Link to open in MockForge Playground

## Configuration

The plugin automatically detects MockForge server at `http://localhost:3000`. To change this:

1. Go to Settings/Preferences → Tools → MockForge
2. Set the server URL

## Building

```bash
# Build the plugin
./gradlew buildPlugin

# Run tests
./gradlew test

# Run IDE with plugin loaded
./gradlew runIde
```

## Development

### Project Structure

```text
jetbrains-plugin/
├── src/
│   ├── main/
│   │   ├── kotlin/com/mockforge/plugin/
│   │   │   ├── MockForgePlugin.kt          # Main plugin class
│   │   │   ├── ConfigValidatorService.kt    # JSON Schema validation
│   │   │   ├── LanguageServer.kt            # Hover, autocomplete
│   │   │   ├── GenerateMockScenarioAction.kt # Code action
│   │   │   ├── MockResponsePreviewProvider.kt # Inline preview
│   │   │   └── inspections/
│   │   │       └── MockForgeConfigInspection.kt
│   │   └── resources/
│   │       └── META-INF/
│   │           └── plugin.xml
│   └── test/
└── build.gradle.kts
```

### Requirements

- IntelliJ Platform SDK 2023.3 or later
- Kotlin 1.9.20 or later
- JDK 17 or later

## Troubleshooting

### Plugin not loading

- Check that you're using a compatible IDE version (2023.3+)
- Check the IDE logs: Help → Show Log in Files

### Config validation not working

- Ensure `mockforge` CLI is installed and in PATH
- Or ensure schema files are bundled in the plugin

### Mock preview not showing

- Ensure MockForge server is running at `http://localhost:3000`
- Check server connection in plugin settings

## Contributing

Contributions are welcome! Please:

1. Open an issue to discuss the implementation approach
2. Follow the patterns established in the VS Code extension
3. Ensure feature parity with VS Code extension where applicable
4. Submit a pull request when ready

## License

MIT OR Apache-2.0
