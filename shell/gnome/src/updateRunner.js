import { ProgressProcess } from './cli.js';
import { afterEvent, afterExit, IDLE, startedRun } from './update.js';

const UPDATE_ARGS = ['update', '--yes'];

export class UpdateRunner {
    constructor(onChange) {
        this._onChange = onChange;
        this._process = null;
        this.run = IDLE;
    }

    start() {
        if (this.run.phase === 'running') return;
        this._set(startedRun());
        try {
            this._process = new ProgressProcess(UPDATE_ARGS, {
                onEvent: event => this._set(afterEvent(this.run, event)),
                onExit: error => this._finish(error),
            });
        } catch (error) {
            this._finish(error.message);
        }
    }

    detach() {
        this._process?.detach();
        this._process = null;
        this._onChange = () => {};
    }

    _finish(errorMessage) {
        this._process = null;
        this._set(afterExit(this.run, errorMessage));
    }

    _set(run) {
        if (run === this.run) return;
        this.run = run;
        this._onChange(run);
    }
}
