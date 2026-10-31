package com.mockforge.plugin

import com.mockforge.plugin.services.MockForgeClientService
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Control-plane credential handling (issue #1011)
 */
class MockForgeClientServiceTest {

    @Test
    fun controlPlaneRequestCarriesTokenWhenSet() {
        val token = "k".repeat(48)
        val request = MockForgeClientService.controlPlaneRequest(
            "http://localhost:3000/__mockforge/api/stats",
            token
        )
        assertEquals(token, request.header(MockForgeClientService.MANAGEMENT_TOKEN_HEADER))
    }

    @Test
    fun controlPlaneRequestOmitsHeaderWhenTokenBlank() {
        val request = MockForgeClientService.controlPlaneRequest(
            "http://localhost:3000/__mockforge/api/stats",
            ""
        )
        assertNull(request.header(MockForgeClientService.MANAGEMENT_TOKEN_HEADER))
    }
}
