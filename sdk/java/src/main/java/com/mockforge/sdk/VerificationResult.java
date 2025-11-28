package com.mockforge.sdk;

import java.util.List;
import java.util.Map;

/**
 * Result of a verification operation
 */
public class VerificationResult {
    /**
     * Whether the verification passed
     */
    private boolean matched;

    /**
     * Actual count of matching requests
     */
    private int count;

    /**
     * Expected count assertion
     */
    private Map<String, Object> expected;

    /**
     * All matching request log entries (for inspection)
     */
    private List<Map<String, Object>> matches;

    /**
     * Error message if verification failed
     */
    private String errorMessage;

    /**
     * Default constructor
     */
    public VerificationResult() {}

    // Getters and setters
    public boolean isMatched() {
        return matched;
    }

    public void setMatched(boolean matched) {
        this.matched = matched;
    }

    public int getCount() {
        return count;
    }

    public void setCount(int count) {
        this.count = count;
    }

    public Map<String, Object> getExpected() {
        return expected;
    }

    public void setExpected(Map<String, Object> expected) {
        this.expected = expected;
    }

    public List<Map<String, Object>> getMatches() {
        return matches;
    }

    public void setMatches(List<Map<String, Object>> matches) {
        this.matches = matches;
    }

    public String getErrorMessage() {
        return errorMessage;
    }

    public void setErrorMessage(String errorMessage) {
        this.errorMessage = errorMessage;
    }
}
