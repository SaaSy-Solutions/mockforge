/**
 * Environment Manager
 *
 * Manages MockForge environments and environment variables
 */

import { MockForgeClient } from './MockForgeClient';
import { Environment, EnvironmentVariable } from '../types';

/**
 * Environment Manager for ForgeConnect SDK
 */
export class EnvironmentManager {
    private client: MockForgeClient;
    private workspaceId?: string;
    private environments: Environment[] = [];
    private activeEnvironment: Environment | null = null;
    private environmentVariables: Record<string, Record<string, string>> = {}; // envId -> variables

    constructor(client: MockForgeClient, workspaceId?: string) {
        this.client = client;
        this.workspaceId = workspaceId;
    }

    /**
     * Initialize and load environments
     */
    async initialize(): Promise<void> {
        await this.refreshEnvironments();
        await this.refreshActiveEnvironment();
    }

    /**
     * Refresh the list of environments
     */
    async refreshEnvironments(): Promise<Environment[]> {
        this.environments = await this.client.listEnvironments(this.workspaceId);
        return this.environments;
    }

    /**
     * Get all environments
     */
    getEnvironments(): Environment[] {
        return [...this.environments];
    }

    /**
     * Get the active environment
     */
    getActiveEnvironment(): Environment | null {
        return this.activeEnvironment;
    }

    /**
     * Refresh the active environment
     */
    async refreshActiveEnvironment(): Promise<Environment | null> {
        this.activeEnvironment = await this.client.getActiveEnvironment(this.workspaceId);
        return this.activeEnvironment;
    }

    /**
     * Set the active environment
     */
    async setActiveEnvironment(environmentId: string): Promise<void> {
        await this.client.setActiveEnvironment(this.workspaceId, environmentId);
        await this.refreshEnvironments();
        await this.refreshActiveEnvironment();
    }

    /**
     * Get environment variables for an environment
     */
    async getEnvironmentVariables(environmentId: string, forceRefresh: boolean = false): Promise<Record<string, string>> {
        if (!forceRefresh && this.environmentVariables[environmentId]) {
            return this.environmentVariables[environmentId];
        }

        const variables = await this.client.getEnvironmentVariables(this.workspaceId, environmentId);
        this.environmentVariables[environmentId] = variables;
        return variables;
    }

    /**
     * Set an environment variable
     */
    async setEnvironmentVariable(environmentId: string, key: string, value: string): Promise<void> {
        await this.client.setEnvironmentVariable(this.workspaceId, environmentId, key, value);

        // Update local cache
        if (!this.environmentVariables[environmentId]) {
            this.environmentVariables[environmentId] = {};
        }
        this.environmentVariables[environmentId][key] = value;
    }

    /**
     * Get variables for the active environment
     */
    async getActiveEnvironmentVariables(forceRefresh: boolean = false): Promise<Record<string, string>> {
        if (!this.activeEnvironment) {
            return {};
        }

        return this.getEnvironmentVariables(this.activeEnvironment.id, forceRefresh);
    }

    /**
     * Set workspace ID
     */
    setWorkspaceId(workspaceId: string | undefined): void {
        this.workspaceId = workspaceId;
        // Clear cached data when workspace changes
        this.environments = [];
        this.activeEnvironment = null;
        this.environmentVariables = {};
    }

    /**
     * Get workspace ID
     */
    getWorkspaceId(): string | undefined {
        return this.workspaceId;
    }
}
