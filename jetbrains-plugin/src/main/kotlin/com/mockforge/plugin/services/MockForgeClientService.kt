package com.mockforge.plugin.services

import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

/**
 * Service for connecting to MockForge server
 *
 * This service:
 * - Manages connection to MockForge server (if running)
 * - Fetches mock responses for inline preview
 * - Handles server connection errors gracefully
 */
@Service(Service.Level.PROJECT)
class MockForgeClientService(private val project: Project) {

    private val client = OkHttpClient.Builder()
        .connectTimeout(2, TimeUnit.SECONDS)
        .readTimeout(2, TimeUnit.SECONDS)
        .build()

    private var serverUrl: String = "http://localhost:3000"
    private var isConnected: Boolean = false

    // Control-plane credential; defaults to the MOCKFORGE_MANAGEMENT_TOKEN env var
    // (the plugin has no settings page yet).
    private var managementToken: String = System.getenv(MANAGEMENT_TOKEN_ENV).orEmpty()

    companion object {
        const val MANAGEMENT_TOKEN_HEADER = "X-MockForge-Management-Token"
        const val MANAGEMENT_TOKEN_ENV = "MOCKFORGE_MANAGEMENT_TOKEN"

        fun getInstance(project: Project): MockForgeClientService = project.service()

        /**
         * Build a control-plane request, attaching the management token when set.
         */
        fun controlPlaneRequest(url: String, managementToken: String): Request {
            val builder = Request.Builder().url(url).get()
            if (managementToken.isNotBlank()) {
                builder.header(MANAGEMENT_TOKEN_HEADER, managementToken)
            }
            return builder.build()
        }
    }

    /**
     * Set MockForge server URL
     */
    fun setServerUrl(url: String) {
        this.serverUrl = url
        this.isConnected = false
    }

    /**
     * Set the management token sent as X-MockForge-Management-Token
     */
    fun setManagementToken(token: String) {
        this.managementToken = token
        this.isConnected = false
    }

    /**
     * Get mock response for an endpoint
     */
    fun getMockResponse(method: String, path: String): MockResponse? {
        if (!isConnected) {
            // Try to connect
            if (!checkConnection()) {
                return null
            }
        }

        return try {
            // Query MockForge API for mock response
            // This would use the MockForge API endpoint
            // For now, return a placeholder
            MockResponse(
                method = method,
                path = path,
                statusCode = 200,
                headers = mapOf("Content-Type" to "application/json"),
                body = "{}"
            )
        } catch (e: Exception) {
            null
        }
    }

    /**
     * Check if MockForge server is running
     */
    fun checkConnection(): Boolean {
        return try {
            val request = controlPlaneRequest("$serverUrl/__mockforge/api/stats", managementToken)

            client.newCall(request).execute().use { response ->
                isConnected = response.isSuccessful
            }
            isConnected
        } catch (e: Exception) {
            isConnected = false
            false
        }
    }

    /**
     * Get connection status
     */
    fun isConnected(): Boolean = isConnected
}

/**
 * Mock response data class
 */
data class MockResponse(
    val method: String,
    val path: String,
    val statusCode: Int,
    val headers: Map<String, String>,
    val body: String
)
