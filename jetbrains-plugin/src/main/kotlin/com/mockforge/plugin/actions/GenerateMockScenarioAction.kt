package com.mockforge.plugin.actions

import com.intellij.codeInsight.intention.IntentionAction
import com.intellij.codeInsight.intention.PriorityAction
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.project.Project
import com.intellij.psi.PsiFile
import com.intellij.psi.util.PsiUtilBase
import org.yaml.snakeyaml.Yaml
import java.io.File
import javax.swing.Icon

/**
 * Intention action to generate MockForge scenario from OpenAPI specification
 *
 * Detects OpenAPI specifications and generates MockForge scenario files
 * from OpenAPI operations
 */
class GenerateMockScenarioAction : IntentionAction, PriorityAction {

    override fun getText(): String = "Generate MockForge Scenario"

    override fun getFamilyName(): String = "MockForge"

    override fun isAvailable(project: Project, editor: Editor?, file: PsiFile?): Boolean {
        if (file == null) return false

        // Check if this looks like an OpenAPI spec
        val content = file.text
        return content.contains("openapi:") ||
               content.contains("\"openapi\"") ||
               content.contains("swagger:") ||
               content.contains("\"swagger\"")
    }

    override fun invoke(project: Project, editor: Editor?, file: PsiFile?) {
        if (file == null || editor == null) return

        try {
            // Parse OpenAPI spec
            val content = file.text
            val spec = parseOpenAPISpec(content, file.name)

            if (spec == null) {
                com.intellij.openapi.ui.Messages.showErrorDialog(
                    project,
                    "Failed to parse OpenAPI specification",
                    "Generate Mock Scenario"
                )
                return
            }

            // Extract operations
            val operations = extractOperations(spec)

            if (operations.isEmpty()) {
                com.intellij.openapi.ui.Messages.showInfoMessage(
                    project,
                    "No operations found in OpenAPI specification",
                    "Generate Mock Scenario"
                )
                return
            }

            // Ask user which operations to generate scenarios for
            val selectedOperations = selectOperations(project, operations)
            if (selectedOperations.isEmpty()) return

            // Ask for scenario name
            val scenarioName = com.intellij.openapi.ui.Messages.showInputDialog(
                project,
                "Enter scenario name:",
                "Generate Mock Scenario",
                com.intellij.icons.AllIcons.General.QuestionDialog,
                "generated-scenario",
                null
            ) ?: return

            // Generate scenario file
            val scenarioContent = generateScenarioYaml(scenarioName, selectedOperations)

            // Save scenario file
            val outputPath = File(file.containingDirectory.virtualFile.path, "$scenarioName.yaml")
            outputPath.writeText(scenarioContent)

            // Open the generated file
            val virtualFile = com.intellij.openapi.vfs.LocalFileSystem.getInstance()
                .refreshAndFindFileByPath(outputPath.absolutePath)
            virtualFile?.let {
                com.intellij.openapi.fileEditor.FileEditorManager.getInstance(project)
                    .openTextEditor(
                        com.intellij.openapi.fileEditor.OpenFileDescriptor(project, it),
                        true
                    )
            }

            com.intellij.openapi.ui.Messages.showInfoMessage(
                project,
                "Generated MockForge scenario: ${outputPath.name}",
                "Generate Mock Scenario"
            )

        } catch (e: Exception) {
            com.intellij.openapi.ui.Messages.showErrorDialog(
                project,
                "Failed to generate scenario: ${e.message}",
                "Generate Mock Scenario"
            )
        }
    }

    override fun startInWriteAction(): Boolean = false

    override fun getPriority(): PriorityAction.Priority = PriorityAction.Priority.NORMAL

    override fun getIcon(): Icon? = null

    /**
     * Parse OpenAPI specification
     */
    private fun parseOpenAPISpec(content: String, fileName: String): Map<*, *>? {
        return try {
            when {
                fileName.endsWith(".json") -> {
                    com.google.gson.Gson().fromJson(content, Map::class.java)
                }
                else -> {
                    // YAML parsing
                    val yaml = Yaml()
                    yaml.load<Map<*, *>>(content)
                }
            }
        } catch (e: Exception) {
            null
        }
    }

    /**
     * Extract operations from OpenAPI spec
     */
    private fun extractOperations(spec: Map<*, *>): List<Operation> {
        val operations = mutableListOf<Operation>()

        val paths = spec["paths"] as? Map<*, *> ?: return emptyList()

        paths.forEach { (path, pathItem) ->
            val pathStr = path.toString()
            val pathItemMap = pathItem as? Map<*, *> ?: return@forEach

            // Extract operations (get, post, put, patch, delete, etc.)
            listOf("get", "post", "put", "patch", "delete", "options", "head").forEach { method ->
                val operation = pathItemMap[method] as? Map<*, *> ?: return@forEach
                val operationId = (operation["operationId"] as? String) ?: "${method}_${pathStr.replace("/", "_")}"
                val summary = (operation["summary"] as? String) ?: ""

                operations.add(Operation(
                    method = method.uppercase(),
                    path = pathStr,
                    operationId = operationId,
                    summary = summary
                ))
            }
        }

        return operations
    }

    /**
     * Select operations to generate scenarios for
     */
    private fun selectOperations(project: Project, operations: List<Operation>): List<Operation> {
        // For now, return all operations
        // In a full implementation, show a dialog to select operations
        return operations
    }

    /**
     * Generate scenario YAML from operations
     */
    private fun generateScenarioYaml(scenarioName: String, operations: List<Operation>): String {
        val yaml = StringBuilder()
        yaml.appendLine("# Generated MockForge Scenario")
        yaml.appendLine("# Auto-generated scenario from OpenAPI specification")
        yaml.appendLine()
        yaml.appendLine("name: $scenarioName")
        yaml.appendLine("description: Auto-generated scenario from OpenAPI specification")
        yaml.appendLine()
        yaml.appendLine("steps:")

        operations.forEachIndexed { index, operation ->
            yaml.appendLine("  - step: ${index + 1}")
            yaml.appendLine("    name: ${operation.summary.ifBlank { operation.operationId }}")
            yaml.appendLine("    method: ${operation.method}")
            yaml.appendLine("    path: ${operation.path}")
            yaml.appendLine("    response:")
            yaml.appendLine("      status: 200")
            yaml.appendLine("      headers:")
            yaml.appendLine("        Content-Type: application/json")
            yaml.appendLine("      body:")
            yaml.appendLine("        {}")
            yaml.appendLine()
        }

        return yaml.toString()
    }

    private data class Operation(
        val method: String,
        val path: String,
        val operationId: String,
        val summary: String
    )
}
