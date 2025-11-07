package com.mockforge.sdk;

import java.util.HashMap;
import java.util.Map;

/**
 * Count assertion for verification
 */
public class VerificationCount {
    /**
     * Request must be made exactly N times
     */
    public static Map<String, Object> exactly(int n) {
        Map<String, Object> result = new HashMap<>();
        result.put("type", "exactly");
        result.put("value", n);
        return result;
    }

    /**
     * Request must be made at least N times
     */
    public static Map<String, Object> atLeast(int n) {
        Map<String, Object> result = new HashMap<>();
        result.put("type", "at_least");
        result.put("value", n);
        return result;
    }

    /**
     * Request must be made at most N times
     */
    public static Map<String, Object> atMost(int n) {
        Map<String, Object> result = new HashMap<>();
        result.put("type", "at_most");
        result.put("value", n);
        return result;
    }

    /**
     * Request must never be made (count must be 0)
     */
    public static Map<String, Object> never() {
        Map<String, Object> result = new HashMap<>();
        result.put("type", "never");
        return result;
    }

    /**
     * Request must be made at least once (count >= 1)
     */
    public static Map<String, Object> atLeastOnce() {
        Map<String, Object> result = new HashMap<>();
        result.put("type", "at_least_once");
        return result;
    }
}
