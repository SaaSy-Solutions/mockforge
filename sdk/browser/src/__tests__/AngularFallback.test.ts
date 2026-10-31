import { vi, it, expect } from 'vitest';
import { ForgeConnectService } from '../adapters/AngularAdapter';


it('provides working observable state when optional RxJS is unavailable', async () => {
    vi.stubGlobal('require', () => { throw new Error('fixture: optional RxJS is unavailable'); });
    const service = new ForgeConnectService({ enableLogging: false });
    const values: boolean[] = [];
    const subscription = service.connected$.subscribe((value: boolean) => values.push(value));
    expect(values).toEqual([false]);
    subscription.unsubscribe();
    await service.stop();
    vi.unstubAllGlobals();
    expect(values).toEqual([false]);
});
