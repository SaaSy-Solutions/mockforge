package com.mockforge.plugin

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.openapi.fileTypes.ex.FileTypeIdentifiersByExtension
import icons.MockForgeIcons
import javax.swing.Icon

/**
 * File type for MockForge configuration files
 *
 * Registers mockforge.yaml, mockforge.yml, and mockforge.toml files
 * as MockForge config files for proper syntax highlighting and validation
 */
class MockForgeConfigFileType : LanguageFileType(MockForgeLanguage.INSTANCE) {

    companion object {
        val INSTANCE = MockForgeConfigFileType()
    }

    override fun getName(): String = "MockForge Config"

    override fun getDescription(): String = "MockForge configuration file"

    override fun getDefaultExtension(): String = "mockforge.yaml"

    override fun getIcon(): Icon? = MockForgeIcons.FILE

    override fun isReadOnly(): Boolean = false
}

/**
 * Language definition for MockForge config files
 *
 * Uses YAML/TOML language support from IntelliJ Platform
 */
class MockForgeLanguage : com.intellij.lang.Language("MockForge") {
    companion object {
        val INSTANCE = MockForgeLanguage()
    }
}
