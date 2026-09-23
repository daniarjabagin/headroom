import Clutter from 'gi://Clutter';
import { clockTime } from '../dates.js';
import { nextUpdateText } from '../format.js';
import { _, fill } from '../i18n.js';
import { isRefreshing } from '../state.js';
import { button, column, iconButton, label, row } from '../widgets.js';

function statusLine(view, now) {
    if (view.kind === 'unavailable') return { text: _('Service not running'), notice: false };
    const state = view.state;
    if (!state) return { text: view.kind === 'loading' ? _('Connecting…') : '', notice: false };
    if (state.offline) {
        const text = state.lastSuccessAt
            ? fill(_('Offline — last update {time}'), { time: clockTime(state.lastSuccessAt) })
            : _('Offline');
        return { text, notice: true };
    }
    if (isRefreshing(state)) return { text: _('Updating…'), notice: false };
    if (state.nextRefreshAt) return { text: nextUpdateText(state.nextRefreshAt, now), notice: false };
    if (state.lastSuccessAt)
        return { text: fill(_('Updated {time}'), { time: clockTime(state.lastSuccessAt) }), notice: false };
    return { text: '', notice: false };
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
        this._statusButton = button(this._status, 'headroom-footer-link', () => ctx.pressRefresh());
        this._statusButton.x_align = Clutter.ActorAlign.START;
        texts.add_child(this._statusButton);
        this.actor.add_child(texts);
        this.preferencesButton = iconButton('emblem-system-symbolic', _('Preferences'), () =>
            ctx.actions.openPreferences()
        );
        ctx.tooltips.attach(this.preferencesButton, () => _('Preferences'));
        this.actor.add_child(this.preferencesButton);
    }

    relabel() {
        this.preferencesButton.accessible_name = _('Preferences');
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
    }
}
