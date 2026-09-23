import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { _, fill } from '../i18n.js';
import { parseProgressLine } from './progress.js';

const SIGTERM = 15;
const KILL_GRACE_SECS = 3;

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
        this._inputClosed = false;
        this._writing = 0;
        this._killTimer = 0;
        this._cancellable = new Gio.Cancellable();
        this._process = Gio.Subprocess.new(
            [binary, ...args, '--progress', 'json'],
            Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_MERGE
        );
        this._stdin = this._process.get_stdin_pipe();
        this._stdout = new Gio.DataInputStream({ base_stream: this._process.get_stdout_pipe() });
        this._process.wait_async(null, (process, result) => this._onReaped(process, result));
        this._readLine();
    }

    write(text) {
        if (this._finished || this._inputClosed) return;
        const bytes = new GLib.Bytes(new TextEncoder().encode(`${text}\n`));
        this._writing += 1;
        this._stdin.write_bytes_async(bytes, GLib.PRIORITY_DEFAULT, this._cancellable, (stream, result) =>
            this._onWritten(stream, result)
        );
    }

    closeInput() {
        if (this._finished || this._inputClosed) return;
        this._inputClosed = true;
        if (this._writing === 0) this._stdin.close(null);
    }

    cancel() {
        if (this._finished) return;
        this._finished = true;
        this._shutdown();
    }

    _onWritten(stream, result) {
        this._writing -= 1;
        try {
            stream.write_bytes_finish(result);
        } catch (error) {
            if (!this._cancellable.is_cancelled()) {
                this._exit(error.message);
                return;
            }
        }
        if ((this._finished || this._inputClosed) && this._writing === 0) this._stdin.close(null);
    }

    _readLine() {
        this._stdout.read_line_async(GLib.PRIORITY_DEFAULT, this._cancellable, (stream, result) => {
            let line;
            try {
                [line] = stream.read_line_finish_utf8(result);
            } catch (error) {
                this._onReadFailed(error);
                return;
            }
            if (line === null) {
                this._stdout.close(null);
                this._waitForExit();
                return;
            }
            const event = parseProgressLine(line);
            if (event) this._handlers.onEvent(event);
            this._readLine();
        });
    }

    _onReadFailed(error) {
        this._stdout.close(null);
        if (!this._cancellable.is_cancelled()) this._exit(error.message);
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
        this._shutdown();
        this._handlers.onExit(errorMessage);
    }

    _shutdown() {
        this._cancellable.cancel();
        if (this._writing === 0) this._stdin.close(null);
        if (this._isRunning()) this._terminate();
    }

    _isRunning() {
        return this._process.get_identifier() !== null;
    }

    _terminate() {
        this._process.send_signal(SIGTERM);
        this._killTimer = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, KILL_GRACE_SECS, () => {
            this._killTimer = 0;
            if (this._isRunning()) this._process.force_exit();
            return GLib.SOURCE_REMOVE;
        });
    }

    _onReaped(process, result) {
        process.wait_finish(result);
        if (this._killTimer !== 0) GLib.source_remove(this._killTimer);
        this._killTimer = 0;
    }
}
