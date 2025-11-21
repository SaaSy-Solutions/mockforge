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
    
    companion object {
        fun getInstance(project: Project): MockForgeClientService = project.service()
    }
    
    /**
     * Set MockForge server URL
     */
    fun setServerUrl(url: String) {
        this.serverUrl = url
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
            val request = Request.Builder()
                .url("$serverUrl/__mockforge/api/stats")
                .get()
                .build()
            
            val response = client.newCall(request).execute()
            isConnected = response.isSuccessful
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

