import Clutter from 'gi://Clutter';
import St from 'gi://St';
import * as Animation from 'resource:///org/gnome/shell/ui/animation.js';
import { clockTime, nextUpdateText } from '../format.js';
import { button, column, label, row, themeIcon } from '../widgets.js';

function statusLine(view, now) {
    if (view.kind === 'unavailable') return { text: 'Service not running', notice: false, busy: false };
    const state = view.state;
    if (!state) return { text: view.kind === 'loading' ? 'Connecting…' : '', notice: false, busy: false };
    if (state.offline) {
        const since = state.lastSuccessAt ? ` — last update ${clockTime(state.lastSuccessAt)}` : '';
        return { text: `Offline${since}`, notice: true, busy: false };
    }
    if (state.accounts.some(account => account.status === 'refreshing'))
        return { text: 'Updating…', notice: false, busy: true };
    if (state.nextRefreshAt) return { text: nextUpdateText(state.nextRefreshAt, now), notice: false, busy: false };
    if (state.generatedAt) return { text: `Updated ${clockTime(state.generatedAt)}`, notice: false, busy: false };
    return { text: '', notice: false, busy: false };
}

export class Footer {
    constructor(ctx, versionText) {
        this._ctx = ctx;
        this._view = { kind: 'loading', state: null };
        this.actor = row({ style_class: 'headroom-footer', x_expand: true });
        const texts = column({
            style_class: 'headroom-footer-texts',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        texts.add_child(label(versionText, 'headroom-footer-text', { x_align: Clutter.ActorAlign.START }));
        this._status = label('', 'headroom-footer-text');
        this._spinner = new Animation.Spinner(10, { animate: true });
        this._spinner.y_align = Clutter.ActorAlign.CENTER;
        const statusRow = row({ style_class: 'headroom-footer-status' });
        statusRow.add_child(this._status);
        statusRow.add_child(this._spinner);
        this._statusButton = button(statusRow, 'headroom-footer-link', () => ctx.actions.refresh(''));
        this._statusButton.x_align = Clutter.ActorAlign.START;
        texts.add_child(this._statusButton);
        this.actor.add_child(texts);
        this.actor.add_child(this._optionsButton());
    }

    _optionsButton() {
        const content = row({ style_class: 'headroom-options-content' });
        content.add_child(new St.Label({ text: 'Options', y_align: Clutter.ActorAlign.CENTER }));
        content.add_child(themeIcon('pan-down-symbolic', 'headroom-options-chevron'));
        this.optionsButton = button(content, 'headroom-options-button', () => this._ctx.actions.toggleOptions());
        return this.optionsButton;
    }

    update(view) {
        this._view = view;
        this.tick(this._ctx.now());
    }

    tick(now) {
        const line = statusLine(this._view, now);
        this._status.text = line.text;
        if (line.notice) this._status.add_style_class_name('notice');
        else this._status.remove_style_class_name('notice');
        this._statusButton.visible = line.text !== '';
        this._statusButton.reactive = this._view.kind === 'ready';
        this._spinner.visible = line.busy;
        if (line.busy) this._spinner.play();
        else this._spinner.stop();
    }
}
