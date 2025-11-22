package com.mockforge.plugin

import com.intellij.codeInsight.completion.*
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.patterns.PlatformPatterns
import com.intellij.psi.PsiElement

/**
 * Completion contributor for MockForge configuration files
 *
 * Provides autocomplete suggestions for configuration keys and values
 * based on JSON Schema definitions
 */
class MockForgeCompletionContributor : CompletionContributor() {

    init {
        // Register completion for YAML and TOML files
        extend(
            CompletionType.BASIC,
            PlatformPatterns.psiElement(),
            MockForgeCompletionProvider()
        )
    }

    private class MockForgeCompletionProvider : com.intellij.codeInsight.completion.CompletionProvider<CompletionParameters>() {
        override fun addCompletions(
            parameters: CompletionParameters,
            context: com.intellij.codeInsight.completion.CompletionContext,
            result: CompletionResultSet
        ) {
            val file = parameters.originalFile
            val element = parameters.position

            // Check if this is a MockForge config file
            val fileName = file.name.lowercase()
            if (!fileName.contains("mockforge") && !fileName.contains("blueprint")) {
                return
            }

            // Get text before cursor
            val textBeforeCursor = getTextBeforeCursor(element)

            // Provide completions based on context
            val completions = when {
                // Top-level keys
                isTopLevelContext(textBeforeCursor) -> getTopLevelCompletions()
                // Reality level enum
                textBeforeCursor.contains("reality_level:") || textBeforeCursor.contains("level:") -> getRealityLevelCompletions()
                // Persona keys
                textBeforeCursor.contains("personas:") -> getPersonaCompletions()
                // Drift budget keys
                textBeforeCursor.contains("drift_budget:") -> getDriftBudgetCompletions()
                else -> emptyList()
            }

            completions.forEach { completion ->
                result.addElement(
                    LookupElementBuilder.create(completion.key)
                        .withTypeText(completion.type)
                        .withTailText(completion.description, true)
                )
            }
        }

        private fun getTextBeforeCursor(element: PsiElement): String {
            val file = element.containingFile
            val offset = element.textOffset
            return file.text.substring(0, offset)
        }

        private fun isTopLevelContext(text: String): Boolean {
            val lines = text.split('\n')
            val lastLine = lines.lastOrNull() ?: return true
            // Check if we're at top level (no indentation or minimal indentation)
            return lastLine.trim().isEmpty() || !lastLine.trimStart().startsWith("  ")
        }

        private fun getTopLevelCompletions(): List<CompletionItem> {
            return listOf(
                CompletionItem("http", "Module", "HTTP server configuration"),
                CompletionItem("websocket", "Module", "WebSocket server configuration"),
                CompletionItem("grpc", "Module", "gRPC server configuration"),
                CompletionItem("admin", "Module", "Admin UI configuration"),
                CompletionItem("reality", "Property", "Reality level configuration"),
                CompletionItem("personas", "Property", "Persona definitions"),
                CompletionItem("drift_budget", "Property", "Drift budget configuration"),
                CompletionItem("observability", "Module", "Observability configuration"),
            )
        }

        private fun getRealityLevelCompletions(): List<CompletionItem> {
            return listOf(
                CompletionItem("static", "Value", "Static stubs - no simulation"),
                CompletionItem("light", "Value", "Light simulation - minimal realism"),
                CompletionItem("moderate", "Value", "Moderate realism - balanced"),
                CompletionItem("high", "Value", "High realism - production-like"),
                CompletionItem("chaos", "Value", "Production chaos - full realism"),
            )
        }

        private fun getPersonaCompletions(): List<CompletionItem> {
            return listOf(
                CompletionItem("id", "Property", "Persona identifier"),
                CompletionItem("name", "Property", "Persona display name"),
                CompletionItem("description", "Property", "Persona description"),
                CompletionItem("traits", "Property", "Persona traits"),
            )
        }

        private fun getDriftBudgetCompletions(): List<CompletionItem> {
            return listOf(
                CompletionItem("enabled", "Property", "Enable drift budget monitoring"),
                CompletionItem("threshold", "Property", "Drift threshold percentage"),
                CompletionItem("window_size", "Property", "Monitoring window size"),
            )
        }
    }

    private data class CompletionItem(
        val key: String,
        val type: String,
        val description: String
    )
}
