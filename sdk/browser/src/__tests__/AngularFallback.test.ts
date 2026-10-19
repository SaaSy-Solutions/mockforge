import { ForgeConnectService } from '../adapters/AngularAdapter';

jest.mock('rxjs', () => { throw new Error('fixture: optional RxJS is unavailable'); }, { virtual: true });

it('provides working observable state when optional RxJS is unavailable', async () => {
    const service = new ForgeConnectService({ enableLogging: false });
    const values: boolean[] = [];
    const subscription = service.connected$.subscribe((value: boolean) => values.push(value));
    expect(values).toEqual([false]);
    subscription.unsubscribe();
    await service.stop();
    expect(values).toEqual([false]);
});
