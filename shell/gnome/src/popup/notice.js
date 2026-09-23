import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { column, row, textButton, themeIcon, wrappingLabel } from '../widgets.js';

const KIND_ICONS = {
    warning: 'dialog-warning-symbolic',
    error: 'dialog-error-symbolic',
    signin: 'system-users-symbolic',
};

function iconTile(kind) {
    const tile = new St.Bin({ style_class: `headroom-notice-tile ${kind}`, y_align: Clutter.ActorAlign.START });
    tile.set_child(themeIcon(KIND_ICONS[kind] ?? KIND_ICONS.warning, 'headroom-notice-icon'));
    return tile;
}

function actionRow(actions) {
    const buttons = row({ style_class: 'headroom-notice-actions', y_align: Clutter.ActorAlign.CENTER });
    for (const action of actions) buttons.add_child(textButton(action.label, 'headroom-small-button', action.run));
    return buttons;
}

export function noticeRow({ kind, title, detail = null, actions = [] }) {
    const actor = row({ style_class: `headroom-notice ${kind}`, x_expand: true });
    const texts = column({ style_class: 'headroom-notice-texts', x_expand: true, y_align: Clutter.ActorAlign.CENTER });
    texts.add_child(wrappingLabel(title, 'headroom-notice-title'));
    if (detail) texts.add_child(wrappingLabel(detail, 'headroom-notice-detail'));
    actor.add_child(iconTile(kind));
    actor.add_child(texts);
    if (actions.length === 1) actor.add_child(actionRow(actions));
    if (actions.length > 1) texts.add_child(actionRow(actions));
    return actor;
}
