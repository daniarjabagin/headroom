import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import { retryBusy, RETRY_FEEDBACK_MS } from '../accountStatus.js';
import { _ } from '../i18n.js';
import { button, column, row, textButton, themeIcon, wrappingLabel } from '../widgets.js';
import { busyIndicator } from './busy.js';

const KIND_ICONS = {
    warning: 'dialog-warning-symbolic',
    error: 'dialog-error-symbolic',
    signin: 'system-users-symbolic',
};
const SPINNER_SIZE = 10;
const COPIED_MS = 1500;
const COPY_ICON = 'edit-copy-symbolic';
const COPIED_ICON = 'object-select-symbolic';

function iconTile(kind) {
    const tile = new St.Bin({ style_class: `headroom-notice-tile ${kind}`, y_align: Clutter.ActorAlign.START });
    tile.set_child(themeIcon(KIND_ICONS[kind] ?? KIND_ICONS.warning, 'headroom-notice-icon'));
    return tile;
}

function actionActor(action) {
    if (action.actor) return action.actor;
    const styleClass = action.primary ? 'headroom-small-button primary' : 'headroom-small-button';
    return textButton(action.label, styleClass, action.run);
}

function actionRow(actions) {
    const buttons = row({ style_class: 'headroom-notice-actions', y_align: Clutter.ActorAlign.CENTER });
    for (const action of actions) buttons.add_child(actionActor(action));
    return buttons;
}

function monotonicMs() {
    return GLib.get_monotonic_time() / 1000;
}

class OneShot {
    constructor(actor) {
        this._id = 0;
        actor.connect('destroy', () => this.clear());
    }

    start(delayMs, run) {
        this.clear();
        this._id = GLib.timeout_add(GLib.PRIORITY_DEFAULT, delayMs, () => {
            this._id = 0;
            run();
            return GLib.SOURCE_REMOVE;
        });
    }

    clear() {
        if (!this._id) return;
        GLib.source_remove(this._id);
        this._id = 0;
    }
}

export class RetryButton {
    constructor(motion, run) {
        this._motion = motion;
        this._busy = false;
        this._refreshing = false;
        this._clickedAt = null;
        this._slot = new St.Bin({ style_class: 'headroom-button-spinner', visible: false });
        this._label = new St.Label({ text: _('Retry'), y_align: Clutter.ActorAlign.CENTER });
        const content = row({ style_class: 'headroom-button-content' });
        content.add_child(this._slot);
        content.add_child(this._label);
        this.actor = button(content, 'headroom-small-button', () => this._onClicked(run));
        this._feedback = new OneShot(this.actor);
    }

    setBusy(refreshing) {
        this._refreshing = refreshing;
        this._render();
    }

    _onClicked(run) {
        this._clickedAt = monotonicMs();
        this._feedback.start(RETRY_FEEDBACK_MS, () => this._render());
        this._render();
        run();
    }

    _render() {
        const busy = retryBusy(this._refreshing, this._clickedAt, monotonicMs());
        if (busy === this._busy) return;
        this._busy = busy;
        this._label.text = busy ? _('Retrying…') : _('Retry');
        this._slot.child?.destroy();
        this._slot.set_child(busy ? busyIndicator(this._motion, SPINNER_SIZE, 'headroom-retry-spinner') : null);
        this._slot.visible = busy;
    }
}

export class CopyButton {
    constructor(copy, text) {
        this._label = new St.Label({ text: _('Copy command'), y_align: Clutter.ActorAlign.CENTER });
        this.actor = button(this._label, 'headroom-small-button primary', () => this._onClicked(copy, text));
        this._reset = new OneShot(this.actor);
    }

    _onClicked(copy, text) {
        copy(text);
        this._label.text = _('Copied');
        this._reset.start(COPIED_MS, () => (this._label.text = _('Copy command')));
    }
}

export class CopyIconButton {
    constructor(copy, text) {
        this._icon = themeIcon(COPY_ICON, 'headroom-small-button-icon');
        this.actor = button(this._icon, 'headroom-small-button icon', () => this._onClicked(copy, text));
        this.actor.accessible_name = _('Copy command');
        this._reset = new OneShot(this.actor);
    }

    _onClicked(copy, text) {
        copy(text);
        this._icon.icon_name = COPIED_ICON;
        this._reset.start(COPIED_MS, () => (this._icon.icon_name = COPY_ICON));
    }
}

export class Notice {
    constructor({ kind, title, detail = null, note = null, actions = [] }) {
        this.actor = row({ style_class: `headroom-notice ${kind}`, x_expand: true });
        const texts = column({
            style_class: 'headroom-notice-texts',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._title = wrappingLabel('', 'headroom-notice-title');
        this._detail = wrappingLabel('', 'headroom-notice-detail');
        this._note = wrappingLabel('', 'headroom-notice-detail headroom-notice-note');
        for (const label of [this._title, this._detail, this._note]) texts.add_child(label);
        this.actor.add_child(iconTile(kind));
        this.actor.add_child(texts);
        if (actions.length === 1) this.actor.add_child(actionRow(actions));
        if (actions.length > 1) texts.add_child(actionRow(actions));
        this.update({ title, detail, note });
    }

    update({ title, detail = null, note = null }) {
        this._title.text = title;
        this._show(this._detail, detail);
        this._show(this._note, note);
    }

    _show(label, text) {
        label.text = text ?? '';
        label.visible = Boolean(text);
    }
}

export function noticeRow(options) {
    return new Notice(options).actor;
}

export class NoticeLine {
    constructor(text) {
        this.actor = row({ style_class: 'headroom-notice-line', x_expand: true });
        const icon = themeIcon('dialog-information-symbolic', 'headroom-notice-line-icon');
        icon.y_align = Clutter.ActorAlign.START;
        this._label = wrappingLabel(text, 'headroom-notice-line-text');
        this.actor.add_child(icon);
        this.actor.add_child(this._label);
    }

    update(text) {
        this._label.text = text;
    }
}

export function noticeLine(text) {
    return new NoticeLine(text).actor;
}
