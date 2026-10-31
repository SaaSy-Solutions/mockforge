import { describe, expect, it, vi } from 'vitest';
import { EnvironmentManager } from '../core/EnvironmentManager';
import { MockForgeClient } from '../core/MockForgeClient';

describe('optional workspace environments', () => {
    it('initializes without contacting workspace endpoints when no workspace is configured', async () => {
        const client = new MockForgeClient('http://localhost:3000');
        const list = vi.spyOn(client, 'listEnvironments');
        const active = vi.spyOn(client, 'getActiveEnvironment');
        const manager = new EnvironmentManager(client);
        await manager.initialize();
        expect(manager.getEnvironments()).toEqual([]);
        expect(manager.getActiveEnvironment()).toBeNull();
        expect(list).not.toHaveBeenCalled();
        expect(active).not.toHaveBeenCalled();
        await expect(manager.setActiveEnvironment('production')).rejects.toThrow('workspaceId');
        await expect(manager.setEnvironmentVariable('production', 'key', 'value')).rejects.toThrow('workspaceId');
        await expect(manager.getEnvironmentVariables('production')).rejects.toThrow('workspaceId');
    });

    it('reports errors for an explicitly configured workspace', async () => {
        const client = new MockForgeClient('http://localhost:3000');
        const list = vi.spyOn(client, 'listEnvironments').mockRejectedValue(new Error('workspace unavailable'));
        const manager = new EnvironmentManager(client, 'configured-workspace');
        await expect(manager.initialize()).rejects.toThrow('workspace unavailable');
        expect(list).toHaveBeenCalledWith('configured-workspace');
    });

    it('clears cached workspace state when workspace configuration is removed', async () => {
        const client = new MockForgeClient('http://localhost:3000');
        const list = vi.spyOn(client, 'listEnvironments').mockResolvedValue([]);
        const active = vi.spyOn(client, 'getActiveEnvironment').mockResolvedValue(null);
        const manager = new EnvironmentManager(client, 'configured-workspace');
        await manager.initialize();
        expect(active).toHaveBeenCalledWith('configured-workspace');
        manager.setWorkspaceId(undefined);
        await manager.initialize();
        expect(list).toHaveBeenCalledTimes(1);
        expect(active).toHaveBeenCalledTimes(1);
        expect(manager.getEnvironments()).toEqual([]);
        expect(manager.getActiveEnvironment()).toBeNull();
    });
});
