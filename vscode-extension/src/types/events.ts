/**
 * WebSocket event types matching the server's MockEvent enum.
 *
 * The server uses serde's internally tagged enum format with snake_case renaming:
 * #[serde(tag = "type", rename_all = "snake_case")]
 */

import { MockConfig, ServerStats } from './mock';

/**
 * Type discriminator for WebSocket events
 */
export type MockEventType = 'mock_created' | 'mock_updated' | 'mock_deleted' | 'stats_updated' | 'connected';

/**
 * Base interface for all WebSocket events
 */
export interface BaseMockEvent {
    /** Event type discriminator */
    type: MockEventType;
    /** ISO 8601 timestamp of the event */
    timestamp: string;
}

/**
 * Event emitted when a mock is created
 */
export interface MockCreatedEvent extends BaseMockEvent {
    type: 'mock_created';
    /** The created mock configuration */
    mock: MockConfig;
}

/**
 * Event emitted when a mock is updated
 */
export interface MockUpdatedEvent extends BaseMockEvent {
    type: 'mock_updated';
    /** The updated mock configuration */
    mock: MockConfig;
}

/**
 * Event emitted when a mock is deleted
 */
export interface MockDeletedEvent extends BaseMockEvent {
    type: 'mock_deleted';
    /** ID of the deleted mock */
    id: string;
}

/**
 * Event emitted when server statistics are updated
 */
export interface StatsUpdatedEvent extends BaseMockEvent {
    type: 'stats_updated';
    /** Updated server statistics */
    stats: ServerStats;
}

/**
 * Event emitted when WebSocket connection is established
 */
export interface ConnectedEvent extends BaseMockEvent {
    type: 'connected';
    /** Connection confirmation message */
    message: string;
}

/**
 * Discriminated union of all possible WebSocket events
 */
export type MockEvent = MockCreatedEvent | MockUpdatedEvent | MockDeletedEvent | StatsUpdatedEvent | ConnectedEvent;
