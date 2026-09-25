import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { button, column, iconButton, label, row, themeIcon } from '../widgets.js';
import { footerLines, footerNeedsSeconds } from './refreshTexts.js';

function setNotice(actor, notice) {
    if (notice) actor.add_style_class_name('notice');
    else actor.remove_style_class_name('notice');
}

export class Footer {
    constructor(ctx, versionText) {
        this._ctx = ctx;
        this._versionText = versionText;
        this._view = { kind: 'loading', state: null };
        this._tip = null;
        this.actor = row({ style_class: 'headroom-footer', x_expand: true });
        const texts = column({
            style_class: 'headroom-footer-texts',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        texts.add_child(this._firstLine());
        texts.add_child(this._secondLine());
        this.actor.add_child(texts);
        this.preferencesButton = iconButton('emblem-system-symbolic', _('Preferences'), () =>
            ctx.actions.openPreferences()
        );
        ctx.tooltips.attach(this.preferencesButton, () => _('Preferences'));
        this.actor.add_child(this.preferencesButton);
    }

    _firstLine() {
        const line = row({ style_class: 'headroom-footer-status', x_align: Clutter.ActorAlign.START });
        this._warning = themeIcon('dialog-warning-symbolic', 'headroom-footer-warning');
        this._first = label(this._versionText, 'headroom-footer-text');
        line.add_child(this._warning);
        line.add_child(this._first);
        this._ctx.tooltips.attach(line, () => (this._first.text === this._versionText ? null : this._versionText));
        return line;
    }

    _secondLine() {
        const content = row({ style_class: 'headroom-footer-status' });
        this._liveDot = new St.Widget({ style_class: 'headroom-live-dot', y_align: Clutter.ActorAlign.CENTER });
        this._status = label('', 'headroom-footer-text');
        content.add_child(this._liveDot);
        content.add_child(this._status);
        this._statusButton = button(content, 'headroom-footer-link', () => this._ctx.pressRefresh());
        this._statusButton.x_align = Clutter.ActorAlign.START;
        this._ctx.tooltips.attach(this._statusButton, () => this._tip);
        return this._statusButton;
    }

    relabel() {
        this.preferencesButton.accessible_name = _('Preferences');
    }

    update(view) {
        this._view = view;
        this.tick(this._ctx.now());
    }

    needsSecondTicks(now) {
        return footerNeedsSeconds(this._view, now);
    }

    tick(now) {
        const lines = footerLines(this._view, now, this._ctx.hour12());
        const first = lines.first ?? { text: this._versionText, kind: 'plain' };
        this._first.text = first.text;
        setNotice(this._first, first.kind === 'notice');
        this._warning.visible = first.kind === 'notice';
        this._status.text = lines.second.text;
        setNotice(this._status, lines.second.kind === 'notice');
        this._liveDot.visible = lines.second.kind === 'live';
        this._tip = lines.tip;
        this._statusButton.visible = lines.second.text !== '';
        this._statusButton.reactive = this._view.kind === 'ready';
    }
}
