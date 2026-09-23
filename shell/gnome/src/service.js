import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { remoteMessage } from './daemonInterface.js';

const UNIT = 'headroom.service';

function callSystemd(method, parameters, cancellable) {
    return new Promise((resolve, reject) => {
        Gio.DBus.session.call(
            'org.freedesktop.systemd1',
            '/org/freedesktop/systemd1',
            'org.freedesktop.systemd1.Manager',
            method,
            parameters,
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            cancellable,
            (connection, result) => {
                try {
                    resolve(connection.call_finish(result));
                } catch (error) {
                    reject(new Error(remoteMessage(error)));
                }
            }
        );
    });
}

export async function startService(cancellable) {
    await callSystemd('StartUnit', new GLib.Variant('(ss)', [UNIT, 'replace']), cancellable);
}
