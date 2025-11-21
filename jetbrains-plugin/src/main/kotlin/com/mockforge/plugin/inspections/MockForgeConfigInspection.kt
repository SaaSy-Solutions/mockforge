package com.mockforge.plugin.inspections

import com.intellij.codeInspection.LocalInspectionTool
import com.intellij.codeInspection.ProblemsHolder
import com.intellij.psi.PsiElementVisitor
import com.intellij.psi.PsiFile
import com.mockforge.plugin.services.ConfigValidatorService
import com.mockforge.plugin.services.SchemaType
import org.jetbrains.yaml.psi.YAMLFile
import org.toml.lang.psi.TomlFile

/**
 * Inspection tool for validating MockForge configuration files
 * 
 * Provides real-time validation of mockforge.yaml, mockforge.toml, and blueprint.yaml files
 * using JSON Schema validation
 */
class MockForgeConfigInspection : LocalInspectionTool() {
    
    override fun buildVisitor(holder: ProblemsHolder, isOnTheFly: Boolean): PsiElementVisitor {
        val file = holder.file
        
        // Check if this is a MockForge config file
        val validator = ConfigValidatorService.getInstance(holder.project)
        val schemaType = when {
            file is YAMLFile -> {
                val fileName = file.name
                val filePath = file.virtualFile?.path ?: ""
                validator.detectSchemaType(fileName, filePath)
            }
            file is TomlFile -> {
                val fileName = file.name
                val filePath = file.virtualFile?.path ?: ""
                if (fileName.contains("mockforge", ignoreCase = true)) {
                    SchemaType.MOCKFORGE_CONFIG
                } else {
                    null
                }
            }
            else -> null
        }
        
        if (schemaType == null) {
            return PsiElementVisitor.EMPTY_VISITOR
        }
        
        return object : PsiElementVisitor() {
            override fun visitFile(file: PsiFile) {
                // Validate the entire file
                val content = file.text
                val errors = validator.validate(content, schemaType)
                
                // Report errors as problems
                errors.forEach { error ->
                    // Find the element at the error location
                    val element = findElementAtLocation(file, error.line, error.column)
                    if (element != null) {
                        holder.registerProblem(
                            element,
                            error.message,
                            com.intellij.codeInspection.ProblemHighlightType.ERROR
                        )
                    } else {
                        // If we can't find the exact element, register on the file
                        holder.registerProblem(
                            file,
                            error.message,
                            com.intellij.codeInspection.ProblemHighlightType.ERROR
                        )
                    }
                }
            }
        }
    }
    
    /**
     * Find PSI element at given line and column
     */
    private fun findElementAtLocation(file: PsiFile, line: Int, column: Int): com.intellij.psi.PsiElement? {
        if (line < 0) return null
        
        val lines = file.text.split('\n')
        if (line >= lines.size) return null
        
        val targetLine = lines[line]
        val offset = lines.take(line).sumOf { it.length + 1 } + column.coerceAtMost(targetLine.length)
        
        return file.findElementAt(offset)
    }
}

