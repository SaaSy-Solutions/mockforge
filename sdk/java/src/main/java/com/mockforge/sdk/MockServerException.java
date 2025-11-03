package com.mockforge.sdk;

/**
 * Exception thrown by MockServer operations
 */
public class MockServerException extends Exception {
    public MockServerException(String message) {
        super(message);
    }

    public MockServerException(String message, Throwable cause) {
        super(message, cause);
    }
}
