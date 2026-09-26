import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { findHeadroom, MissingBinaryError } from '../cli.js';

export function loginArgv(binary, accountId) {
    return [binary, 'accounts', 'login', accountId];
}

export class LoginLauncher {
    constructor(onFailure) {
        this._onFailure = onFailure;
        this._cancellable = new Gio.Cancellable();
    }

    launch(accountId) {
        const binary = findHeadroom();
        if (!binary) {
            this._onFailure(new MissingBinaryError().message);
            return;
        }
        try {
            const process = Gio.Subprocess.new(loginArgv(binary, accountId), Gio.SubprocessFlags.STDOUT_SILENCE);
            process.wait_check_async(this._cancellable, (child, result) => this._onExit(child, result));
        } catch (error) {
            if (!(error instanceof GLib.Error)) throw error;
            this._onFailure(error.message);
        }
    }

    _onExit(child, result) {
        try {
            child.wait_check_finish(result);
        } catch (error) {
            if (!(error instanceof GLib.Error)) throw error;
            if (!this._cancellable.is_cancelled()) this._onFailure(error.message);
        }
    }

    destroy() {
        this._cancellable.cancel();
    }
}
