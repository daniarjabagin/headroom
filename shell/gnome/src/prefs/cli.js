import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { _, fill } from '../i18n.js';
import { parseProgressLine } from './progress.js';

export function findHeadroom() {
    const onPath = GLib.find_program_in_path('headroom');
    if (onPath) return onPath;
    const local = GLib.build_filenamev([GLib.get_home_dir(), '.local', 'bin', 'headroom']);
    return GLib.file_test(local, GLib.FileTest.IS_EXECUTABLE) ? local : null;
}

export class MissingBinaryError extends Error {
    constructor() {
        super(_("Couldn't find the headroom command. Install Headroom or add it to your PATH."));
    }
}

export class ProgressProcess {
    constructor(args, { onEvent, onExit }) {
        const binary = findHeadroom();
        if (!binary) throw new MissingBinaryError();
        this._handlers = { onEvent, onExit };
        this._finished = false;
        this._cancellable = new Gio.Cancellable();
        this._process = Gio.Subprocess.new(
            [binary, ...args, '--progress', 'json'],
            Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_MERGE
        );
        this._stdout = new Gio.DataInputStream({ base_stream: this._process.get_stdout_pipe() });
        this._readLine();
    }

    write(text) {
        const bytes = new GLib.Bytes(new TextEncoder().encode(`${text}\n`));
        this._process.get_stdin_pipe().write_bytes_async(bytes, GLib.PRIORITY_DEFAULT, this._cancellable, null);
    }

    cancel() {
        if (this._finished) return;
        this._finished = true;
        this._cancellable.cancel();
        this._process.force_exit();
    }

    _readLine() {
        this._stdout.read_line_async(GLib.PRIORITY_DEFAULT, this._cancellable, (stream, result) => {
            let line;
            try {
                [line] = stream.read_line_finish_utf8(result);
            } catch (error) {
                if (!this._cancellable.is_cancelled()) this._exit(error.message);
                return;
            }
            if (line === null) {
                this._waitForExit();
                return;
            }
            const event = parseProgressLine(line);
            if (event) this._handlers.onEvent(event);
            this._readLine();
        });
    }

    _waitForExit() {
        this._process.wait_async(this._cancellable, (process, result) => {
            try {
                process.wait_finish(result);
            } catch (error) {
                if (!this._cancellable.is_cancelled()) this._exit(error.message);
                return;
            }
            const failed = !process.get_successful();
            this._exit(failed ? fill(_('headroom exited with status {status}'), { status: this._status() }) : null);
        });
    }

    _status() {
        return this._process.get_if_exited() ? this._process.get_exit_status() : this._process.get_term_sig();
    }

    _exit(errorMessage) {
        if (this._finished) return;
        this._finished = true;
        this._handlers.onExit(errorMessage);
    }
}
