import GLib from 'gi://GLib';
import { PrefsClient } from '../src/prefs/client.js';
import { check } from './check.js';

const SETTINGS_JSON = '{"refresh_interval_secs": 300}';

const fakeProxy = {
    GetStateAsync: () => Promise.resolve(['{"version": 1}']),
    GetSettingsAsync: () => Promise.resolve([SETTINGS_JSON]),
    ListProvidersAsync: () => Promise.resolve(['{"version": 1, "providers": []}']),
};

class FakeConnection {
    constructor(options) {
        this.options = options;
        this.proxy = null;
        this.pending = Promise.resolve();
        this.enqueued = 0;
        this.destroyed = false;
    }

    call(invoke) {
        this.pending = Promise.resolve(invoke(this.proxy));
        return this.pending;
    }

    enqueue() {
        this.enqueued += 1;
        return Promise.resolve(true);
    }

    destroy() {
        this.destroyed = true;
    }

    async appear() {
        this.proxy = fakeProxy;
        this.options.onReady(fakeProxy);
        await this.pending;
    }
}

function drainMainContext() {
    const context = GLib.MainContext.default();
    while (context.pending()) context.iteration(false);
}

async function readyClient(delivered) {
    let connection = null;
    const client = new PrefsClient({
        onAvailable: () => {},
        onUnavailable: () => {},
        onState: () => {},
        onSettings: settings => delivered.push(settings.refreshIntervalSecs),
        onError: message => delivered.push(`error: ${message}`),
        connect: options => (connection = new FakeConnection(options)),
    });
    await connection.appear();
    return { client, connection };
}

async function testOptimisticSettingsDeferred() {
    const delivered = [];
    const { client, connection } = await readyClient(delivered);
    check('loaded settings delivered', delivered, [300]);
    client.updateSettings({ refresh_interval_secs: 60 });
    check('optimistic settings not delivered synchronously', delivered, [300]);
    check('optimistic settings readable at once', client.settings.refreshIntervalSecs, 60);
    check('patch sent', connection.enqueued, 1);
    client.updateSettings({ refresh_interval_secs: 120 });
    drainMainContext();
    check('optimistic settings delivered once on idle', delivered, [300, 120]);
    client.destroy();
}

async function testDestroyCancelsDelivery() {
    const delivered = [];
    const { client, connection } = await readyClient(delivered);
    client.updateSettings({ refresh_interval_secs: 60 });
    client.destroy();
    drainMainContext();
    check('no delivery after destroy', delivered, [300]);
    check('connection destroyed', connection.destroyed, true);
}

export async function testPrefsClient() {
    await testOptimisticSettingsDeferred();
    await testDestroyCancelsDelivery();
}
