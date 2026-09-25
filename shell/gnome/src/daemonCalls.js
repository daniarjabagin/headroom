import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { remoteMessage } from './daemonInterface.js';
import { parseDiagnostics } from './diagnostics.js';
import { _ } from './i18n.js';
import { parseSpendResult, spendQuery } from './spendQuery.js';

export const SPEND_TIMEOUT_MS = 15_000;
export const DIAGNOSTICS_TIMEOUT_MS = 10_000;
export const RESET_TIMEOUT_MS = 10_000;

export class DaemonCallError extends Error {
    constructor(message, remoteName = null) {
        super(message);
        this.remoteName = remoteName;
    }
}

function finished(proxy, response) {
    try {
        return proxy.call_finish(response).deepUnpack();
    } catch (error) {
        if (!(error instanceof GLib.Error)) throw error;
        if (error.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED)) return null;
        throw new DaemonCallError(remoteMessage(error), Gio.DBusError.get_remote_error(error));
    }
}

export function callDaemon(proxy, method, parameters, timeoutMs, cancellable) {
    if (!proxy) return Promise.reject(new DaemonCallError(_("Headroom service isn't running")));
    return new Promise((resolve, reject) => {
        proxy.call(method, parameters, Gio.DBusCallFlags.NO_AUTO_START, timeoutMs, cancellable, (source, response) => {
            try {
                resolve(finished(source, response));
            } catch (error) {
                reject(error);
            }
        });
    });
}

export async function requestSpend(proxy, options, cancellable) {
    const parameters = new GLib.Variant('(s)', [JSON.stringify(spendQuery(options))]);
    const reply = await callDaemon(proxy, 'GetSpend', parameters, SPEND_TIMEOUT_MS, cancellable);
    return reply ? parseSpendResult(reply[0]) : null;
}

export async function requestDiagnostics(proxy, cancellable) {
    const reply = await callDaemon(proxy, 'GetDiagnostics', null, DIAGNOSTICS_TIMEOUT_MS, cancellable);
    return reply ? parseDiagnostics(reply[0]) : null;
}

export async function requestReset(proxy, cancellable) {
    const reply = await callDaemon(proxy, 'ResetSettings', null, RESET_TIMEOUT_MS, cancellable);
    return reply !== null;
}
