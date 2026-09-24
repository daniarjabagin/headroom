import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { button, column, row, textButton, themeIcon, wrappingLabel } from '../widgets.js';
import { busyIndicator } from './busy.js';

const KIND_ICONS = {
    warning: 'dialog-warning-symbolic',
    error: 'dialog-error-symbolic',
    signin: 'system-users-symbolic',
};
const SPINNER_SIZE = 10;

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

export class RetryButton {
    constructor(motion, run) {
        this._motion = motion;
        this._busy = false;
        this._slot = new St.Bin({ style_class: 'headroom-button-spinner', visible: false });
        this._label = new St.Label({ text: _('Retry'), y_align: Clutter.ActorAlign.CENTER });
        const content = row({ style_class: 'headroom-button-content' });
        content.add_child(this._slot);
        content.add_child(this._label);
        this.actor = button(content, 'headroom-small-button', run);
    }

    setBusy(busy) {
        if (busy === this._busy) return;
        this._busy = busy;
        this.actor.reactive = !busy;
        this.actor.can_focus = !busy;
        this._label.text = busy ? _('Retrying…') : _('Retry');
        this._slot.child?.destroy();
        this._slot.set_child(busy ? busyIndicator(this._motion, SPINNER_SIZE, 'headroom-retry-spinner') : null);
        this._slot.visible = busy;
        this.actor.sync_hover();
    }
}

export function noticeRow({ kind, title, detail = null, note = null, actions = [] }) {
    const actor = row({ style_class: `headroom-notice ${kind}`, x_expand: true });
    const texts = column({ style_class: 'headroom-notice-texts', x_expand: true, y_align: Clutter.ActorAlign.CENTER });
    texts.add_child(wrappingLabel(title, 'headroom-notice-title'));
    if (detail) texts.add_child(wrappingLabel(detail, 'headroom-notice-detail'));
    if (note) texts.add_child(wrappingLabel(note, 'headroom-notice-detail headroom-notice-note'));
    actor.add_child(iconTile(kind));
    actor.add_child(texts);
    if (actions.length === 1) actor.add_child(actionRow(actions));
    if (actions.length > 1) texts.add_child(actionRow(actions));
    return actor;
}
