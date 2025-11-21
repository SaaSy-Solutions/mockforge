package com.mockforge.plugin

import com.intellij.lang.documentation.DocumentationProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiManager

/**
 * Documentation provider for MockForge configuration files
 * 
 * Provides hover documentation for configuration keys
 */
class MockForgeDocumentationProvider : DocumentationProvider {
    
    override fun getQuickNavigateInfo(element: PsiElement, originalElement: PsiElement): String? {
        return getDocumentation(element)
    }
    
    override fun generateDoc(element: PsiElement, originalElement: PsiElement?): String? {
        return getDocumentation(element)
    }
    
    private fun getDocumentation(element: PsiElement): String? {
        val text = element.text
        
        // Check if this is a config key
        val key = extractConfigKey(text)
        if (key == null) return null
        
        // Return documentation for the key
        return getKeyDocumentation(key)
    }
    
    private fun extractConfigKey(text: String): String? {
        // Extract key from YAML/TOML format
        val trimmed = text.trim()
        return when {
            trimmed.endsWith(":") -> trimmed.dropLast(1).trim()
            trimmed.contains("=") -> trimmed.split("=")[0].trim()
            else -> null
        }
    }
    
    private fun getKeyDocumentation(key: String): String? {
        val docs = mapOf(
            "reality_level" to "Reality level controls how realistic mock responses are. Values: static, light, moderate, high, chaos",
            "reality" to "Reality configuration for unified realism control",
            "personas" to "Persona definitions for consistent, personality-driven data generation",
            "drift_budget" to "Contract drift budget configuration for monitoring API changes",
            "http" to "HTTP server configuration",
            "websocket" to "WebSocket server configuration",
            "grpc" to "gRPC server configuration",
            "admin" to "Admin UI configuration",
            "observability" to "Metrics, tracing, and observability configuration",
        )
        
        return docs[key]
    }
}

