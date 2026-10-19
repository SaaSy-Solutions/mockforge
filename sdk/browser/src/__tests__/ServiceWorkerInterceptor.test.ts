import { ServiceWorkerInterceptor } from '../core/ServiceWorkerInterceptor';

describe('ServiceWorker capture callbacks', () => {
    const original = Object.getOwnPropertyDescriptor(navigator, 'serviceWorker');

    afterEach(() => {
        if (original) Object.defineProperty(navigator, 'serviceWorker', original);
        else Reflect.deleteProperty(navigator, 'serviceWorker');
        jest.restoreAllMocks();
    });

    it.each([
        ['synchronous', () => undefined, false],
        ['rejected promise', () => Promise.reject(new Error('fixture rejection')), true],
        ['synchronous exception', () => { throw new Error('fixture exception'); }, true],
    ] as const)('handles a %s callback', async (_name, implementation, fails) => {
        const worker = {
            register: jest.fn().mockResolvedValue({ unregister: jest.fn().mockResolvedValue(true) }),
            ready: Promise.resolve(),
            addEventListener: jest.fn(),
        };
        Object.defineProperty(navigator, 'serviceWorker', { configurable: true, value: worker });
        const warn = jest.spyOn(console, 'warn').mockImplementation(() => {});
        jest.spyOn(console, 'log').mockImplementation(() => {});
        const callback = jest.fn(implementation);
        const interceptor = new ServiceWorkerInterceptor();
        expect(await interceptor.start(callback)).toBe(true);
        const receive = worker.addEventListener.mock.calls.find(([event]) => event === 'message')![1];
        const request = { method: 'GET', url: '/callback-control', path: '/callback-control', timestamp: 1 };
        receive({ data: { type: 'FORGECONNECT_REQUEST', payload: request } });
        await new Promise(resolve => setTimeout(resolve, 0));
        expect(callback).toHaveBeenCalledWith(request);
        expect(warn).toHaveBeenCalledTimes(fails ? 1 : 0);
        await interceptor.stop();
    });
});
