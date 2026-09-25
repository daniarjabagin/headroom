import Clutter from 'gi://Clutter';
import { EASE, STANDARD_MS } from '../motion.js';

export class NumberTween {
    constructor(motion, label, render) {
        this._motion = motion;
        this._label = label;
        this._render = render;
        this._shown = null;
        this._target = null;
        this._timeline = null;
        label.connect('destroy', () => this._stop());
    }

    set(value, animate) {
        if (this._timeline && value === this._target) return;
        const from = this._shown;
        this._stop();
        this._target = value;
        if (!animate || from === null || from === value || !this._motion.enabled || !this._label.mapped) {
            this._show(value);
            return;
        }
        this._timeline = new Clutter.Timeline({ actor: this._label, duration: STANDARD_MS, progress_mode: EASE });
        this._timeline.connect('new-frame', timeline => this._show(from + (value - from) * timeline.get_progress()));
        this._timeline.connect('completed', () => {
            this._timeline = null;
            this._show(value);
        });
        this._timeline.start();
    }

    _show(value) {
        this._shown = value;
        this._label.text = this._render(value);
    }

    _stop() {
        if (!this._timeline) return;
        this._timeline.stop();
        this._timeline = null;
    }
}
