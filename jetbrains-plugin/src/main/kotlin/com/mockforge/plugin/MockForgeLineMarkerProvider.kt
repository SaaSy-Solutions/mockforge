package com.mockforge.plugin

import com.intellij.codeInsight.daemon.LineMarkerInfo
import com.intellij.codeInsight.daemon.LineMarkerProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.util.PsiTreeUtil
import com.mockforge.plugin.services.MockForgeClientService

/**
 * Line marker provider for showing mock response preview
 *
 * Detects endpoint references in code and provides hover tooltips
 * with mock response preview
 */
class MockForgeLineMarkerProvider : LineMarkerProvider {

    override fun getLineMarkerInfo(element: PsiElement): LineMarkerInfo<*>? {
        // Extract endpoint information from code
        val endpoint = extractEndpoint(element)
        if (endpoint == null) return null

        // Create line marker with hover tooltip
        return LineMarkerInfo(
            element,
            element.textRange,
            com.intellij.icons.AllIcons.General.Information,
            { "MockForge: ${endpoint.method} ${endpoint.path}" },
            { _, _ -> showMockPreview(element.project, endpoint) },
            com.intellij.codeInsight.daemon.GutterIconRenderer.Alignment.LEFT
        )
    }

    /**
     * Extract endpoint information from code element
     */
    private fun extractEndpoint(element: PsiElement): Endpoint? {
        val text = element.text

        // Pattern 1: HTTP method followed by URL string
        // Examples: fetch('/api/users'), axios.get('/api/users'), http.get('/api/users')
        val httpMethodPattern = Regex("""(?:fetch|axios|http)\.?(get|post|put|patch|delete|options|head)\s*\(['"`]([^'"`]+)['"`]""", RegexOption.IGNORE_CASE)
        val methodMatch = httpMethodPattern.find(text)
        if (methodMatch != null) {
            return Endpoint(
                method = methodMatch.groupValues[1].uppercase(),
                path = methodMatch.groupValues[2]
            )
        }

        // Pattern 2: URL string with method in comment or nearby
        val urlPattern = Regex("""['"`]([/][^'"`]+)['"`]""")
        val urlMatch = urlPattern.find(text)
        if (urlMatch != null) {
            // Check previous line for HTTP method
            val prevElement = PsiTreeUtil.getPrevSiblingOfType(element, PsiElement::class.java)
            if (prevElement != null) {
                val methodMatch = Regex("""\b(GET|POST|PUT|PATCH|DELETE|OPTIONS|HEAD)\b""", RegexOption.IGNORE_CASE)
                    .find(prevElement.text)
                if (methodMatch != null) {
                    return Endpoint(
                        method = methodMatch.groupValues[1].uppercase(),
                        path = urlMatch.groupValues[1]
                    )
                }
            }
            // Default to GET if no method found
            return Endpoint(
                method = "GET",
                path = urlMatch.groupValues[1]
            )
        }

        // Pattern 3: REST client patterns (axios, fetch without method)
        val fetchPattern = Regex("""fetch\s*\(['"`]([^'"`]+)['"`]""", RegexOption.IGNORE_CASE)
        val fetchMatch = fetchPattern.find(text)
        if (fetchMatch != null) {
            return Endpoint(
                method = "GET", // fetch defaults to GET
                path = fetchMatch.groupValues[1]
            )
        }

        return null
    }

    /**
     * Show mock preview in a tooltip or popup
     */
    private fun showMockPreview(project: com.intellij.openapi.project.Project, endpoint: Endpoint) {
        val client = MockForgeClientService.getInstance(project)
        val response = client.getMockResponse(endpoint.method, endpoint.path)

        if (response != null) {
            val message = buildString {
                appendLine("Mock Response Preview")
                appendLine("${endpoint.method} ${endpoint.path}")
                appendLine()
                appendLine("Status: ${response.statusCode}")
                appendLine("Headers: ${response.headers}")
                appendLine("Body: ${response.body}")
            }

            com.intellij.openapi.ui.Messages.showInfoMessage(
                project,
                message,
                "MockForge Preview"
            )
        } else {
            com.intellij.openapi.ui.Messages.showInfoMessage(
                project,
                "No mock configured for ${endpoint.method} ${endpoint.path}",
                "MockForge Preview"
            )
        }
    }

    private data class Endpoint(
        val method: String,
        val path: String
    )
}
