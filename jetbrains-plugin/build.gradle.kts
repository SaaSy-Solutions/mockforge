plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "1.9.20"
    id("org.jetbrains.intellij") version "1.16.0"
}

group = "com.mockforge"
version = "0.1.0"

repositories {
    mavenCentral()
}

// Configure IntelliJ Platform plugin
intellij {
    version.set("2023.3")
    type.set("IC") // IntelliJ IDEA Community Edition

    // Plugin name and description
    plugins.set(listOf("yaml", "com.intellij.toml"))
}

dependencies {
    // Kotlin standard library
    implementation("org.jetbrains.kotlin:kotlin-stdlib-jdk8")

    // JSON Schema validation
    implementation("com.networknt:json-schema-validator:1.0.76")

    // YAML parsing
    implementation("org.yaml:snakeyaml:2.2")

    // TOML parsing
    implementation("com.moandjiezana.toml:toml4j:0.7.2")

    // HTTP client for connecting to MockForge server
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    // JSON processing
    implementation("com.google.code.gson:gson:2.10.1")
    implementation("com.fasterxml.jackson.core:jackson-databind:2.15.2")

    // Testing
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit")
}

tasks {
    // Set JVM target
    withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
        kotlinOptions.jvmTarget = "17"
    }

    patchPluginXml {
        sinceBuild.set("233")
        untilBuild.set("241.*")

        // Plugin description
        pluginDescription.set("""
            MockForge IDE integration for JetBrains IDEs.

            Features:
            - Config validation for mockforge.yaml and mockforge.toml files
            - Autocomplete for configuration keys and values
            - Generate Mock Scenario code action for OpenAPI specifications
            - Inline preview of mock responses when hovering over endpoint references
            - Real-time linting for MockForge configuration files
        """.trimIndent())

        // Change notes
        changeNotes.set("""
            <h3>Initial Release</h3>
            <ul>
                <li>Config validation using JSON Schema</li>
                <li>Autocomplete for MockForge configuration files</li>
                <li>Generate Mock Scenario code action</li>
                <li>Inline preview of mock responses</li>
            </ul>
        """.trimIndent())
    }

    signPlugin {
        certificateChain.set(System.getenv("CERTIFICATE_CHAIN"))
        privateKey.set(System.getenv("PRIVATE_KEY"))
        password.set(System.getenv("PRIVATE_KEY_PASSWORD"))
    }

    publishPlugin {
        token.set(System.getenv("PUBLISH_TOKEN"))
    }
}
