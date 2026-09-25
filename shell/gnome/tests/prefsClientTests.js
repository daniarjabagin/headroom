import Gio from 'gi://Gio';
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

function answering(reply) {
    const calls = [];
    const proxy = {
        ...fakeProxy,
        call: (method, parameters, flags, timeout, cancellable, callback) => {
            calls.push({ method, timeout });
            callback(proxy, reply);
        },
        call_finish: response => {
            if (response instanceof GLib.Error) throw response;
            return new GLib.Variant('(s)', [response]);
        },
    };
    return { proxy, calls };
}

async function checkWith(reply) {
    const { client, connection } = await readyClient([]);
    const { proxy, calls } = answering(reply);
    connection.proxy = proxy;
    const result = await client.checkForUpdates();
    client.destroy();
    return { result, calls };
}

async function testCheckForUpdates() {
    const { result, calls } = await checkWith(
        '{"status":"up_to_date","checked_at":"2026-09-23T10:00:00Z","version":"0.6.0"}'
    );
    check('check method', calls[0].method, 'CheckForUpdates');
    check('check waits at least a minute', calls[0].timeout >= 60_000, true);
    check('check result', [result.status, result.version], ['up_to_date', '0.6.0']);
    const unsupported = Gio.DBusError.new_for_dbus_error('org.freedesktop.DBus.Error.NotSupported', 'off');
    check('check not supported', (await checkWith(unsupported)).result.status, 'unsupported');
    const failed = Gio.DBusError.new_for_dbus_error('org.freedesktop.DBus.Error.Failed', 'shutting down');
    check('check failed', (await checkWith(failed)).result.status, 'failed');
    const cancelled = new GLib.Error(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED, 'cancelled');
    check('check cancelled', (await checkWith(cancelled)).result, null);
}

async function testResetSettings() {
    const delivered = [];
    const { client, connection } = await readyClient(delivered);
    const { proxy, calls } = answering('');
    connection.proxy = proxy;
    check('reset done', await client.resetSettings(), true);
    await connection.pending;
    check('reset method', calls[0].method, 'ResetSettings');
    check('reset reloads settings', delivered, [300, 300]);
    client.destroy();
}

async function testDiagnostics() {
    const { client, connection } = await readyClient([]);
    connection.proxy = answering('{"app_version":"0.6.0","text":"Headroom 0.6.0\\n"}').proxy;
    const report = await client.getDiagnostics();
    check('diagnostics report', [report.appVersion, report.text], ['0.6.0', 'Headroom 0.6.0\n']);
    const unknown = Gio.DBusError.new_for_dbus_error('org.freedesktop.DBus.Error.UnknownMethod', 'no such method');
    connection.proxy = answering(unknown).proxy;
    try {
        await client.getDiagnostics();
        check('diagnostics on old daemon throws', false, true);
    } catch (error) {
        check(
            'diagnostics on old daemon',
            [error.constructor.name, error.remoteName],
            ['DaemonCallError', 'org.freedesktop.DBus.Error.UnknownMethod']
        );
    }
    client.destroy();
}

export async function testPrefsClient() {
    await testOptimisticSettingsDeferred();
    await testDestroyCancelsDelivery();
    await testCheckForUpdates();
    await testResetSettings();
    await testDiagnostics();
}
