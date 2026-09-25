import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { remoteMessage } from '../daemonInterface.js';

const UNIT = 'headroom.service';
const SYSTEMD = 'org.freedesktop.systemd1';
const MANAGER_PATH = '/org/freedesktop/systemd1';
const MANAGER = 'org.freedesktop.systemd1.Manager';

export class SystemdError extends Error {}

function call({ path = MANAGER_PATH, iface = MANAGER, method, parameters = null, cancellable }) {
    return new Promise((resolve, reject) => {
        Gio.DBus.session.call(
            SYSTEMD,
            path,
            iface,
            method,
            parameters,
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            cancellable,
            (connection, result) => {
                try {
                    resolve(connection.call_finish(result).deepUnpack());
                } catch (error) {
                    reject(new SystemdError(remoteMessage(error)));
                }
            }
        );
    });
}

export async function restartService(cancellable) {
    await call({ method: 'RestartUnit', parameters: new GLib.Variant('(ss)', [UNIT, 'replace']), cancellable });
}

export async function serviceActive(cancellable) {
    try {
        const [path] = await call({ method: 'GetUnit', parameters: new GLib.Variant('(s)', [UNIT]), cancellable });
        const [state] = await call({
            path,
            iface: 'org.freedesktop.DBus.Properties',
            method: 'Get',
            parameters: new GLib.Variant('(ss)', ['org.freedesktop.systemd1.Unit', 'ActiveState']),
            cancellable,
        });
        return state.deepUnpack() === 'active';
    } catch (error) {
        if (error instanceof SystemdError) return false;
        throw error;
    }
}

export async function serviceEnabled(cancellable) {
    try {
        const [state] = await call({
            method: 'GetUnitFileState',
            parameters: new GLib.Variant('(s)', [UNIT]),
            cancellable,
        });
        return state === 'enabled';
    } catch (error) {
        if (error instanceof SystemdError) return null;
        throw error;
    }
}

export async function setServiceEnabled(enabled, cancellable) {
    const parameters = enabled
        ? new GLib.Variant('(asbb)', [[UNIT], false, false])
        : new GLib.Variant('(asb)', [[UNIT], false]);
    await call({ method: enabled ? 'EnableUnitFiles' : 'DisableUnitFiles', parameters, cancellable });
    await call({ method: 'Reload', cancellable });
}
