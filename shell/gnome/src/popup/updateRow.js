import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { IDLE, runLine, showsWhatsNew, updateAction, updateTitle } from '../update.js';
import { button, column, label, row, themeIcon, wrappingLabel } from '../widgets.js';
import { busyIndicator } from './busy.js';

const SPINNER_SIZE = 10;
const BADGE_ICONS = { done: 'object-select-symbolic', failed: 'dialog-error-symbolic' };

function badgeActor(motion, phase) {
    if (phase === 'running') return busyIndicator(motion, SPINNER_SIZE, 'headroom-update-badge');
    const icon = BADGE_ICONS[phase];
    return icon ? themeIcon(icon, `headroom-update-badge ${phase}`) : null;
}

function setClass(actor, name, enabled) {
    if (enabled) actor.add_style_class_name(name);
    else actor.remove_style_class_name(name);
}

export class UpdateRow {
    constructor(ctx) {
        this._ctx = ctx;
        this._update = null;
        this._run = IDLE;
        this._badgePhase = null;
        this._expanded = false;
        this._copied = false;
        this.actor = column({ style_class: 'headroom-update', x_expand: true, visible: false });
        this.actor.add_child(this._buildMain());
        this.actor.add_child(this._buildCommand());
    }

    update(update) {
        if (update?.version !== this._update?.version || update?.command !== this._update?.command) {
            this._expanded = false;
            this._copied = false;
        }
        this._update = update;
        this._sync();
    }

    setRun(run) {
        this._run = run;
        this._sync();
    }

    relabel() {
        this._sync();
    }

    _buildMain() {
        const main = row({ style_class: 'headroom-update-main', x_expand: true });
        const texts = column({
            style_class: 'headroom-update-texts',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._title = wrappingLabel('', 'headroom-update-title');
        texts.add_child(this._title);
        texts.add_child(this._buildLine());
        main.add_child(themeIcon('software-update-available-symbolic', 'headroom-update-icon'));
        main.add_child(texts);
        this._actionLabel = new St.Label({ y_align: Clutter.ActorAlign.CENTER });
        this._action = button(this._actionLabel, 'headroom-small-button', () => this._onAction());
        main.add_child(this._action);
        return main;
    }

    _buildLine() {
        const line = row({ style_class: 'headroom-update-line', x_expand: true });
        this._badge = new St.Bin({ style_class: 'headroom-update-badge-slot', visible: false });
        this._badge.y_align = Clutter.ActorAlign.START;
        this._detail = wrappingLabel('', 'headroom-update-detail');
        this._whatsNewLabel = label('', 'headroom-update-link-text');
        this._whatsNew = button(this._whatsNewLabel, 'headroom-update-link', () => this._openRelease());
        this._whatsNew.x_align = Clutter.ActorAlign.START;
        line.add_child(this._badge);
        line.add_child(this._detail);
        line.add_child(this._whatsNew);
        return line;
    }

    _buildCommand() {
        this._command = row({ style_class: 'headroom-update-command', x_expand: true, visible: false });
        this._commandText = wrappingLabel('', 'headroom-update-command-text');
        this._copyLabel = new St.Label({ y_align: Clutter.ActorAlign.CENTER });
        this._copy = button(this._copyLabel, 'headroom-small-button', () => this._copyCommand());
        this._copy.y_align = Clutter.ActorAlign.START;
        this._command.add_child(this._commandText);
        this._command.add_child(this._copy);
        return this._command;
    }

    _sync() {
        const update = this._update;
        this.actor.visible = update !== null;
        if (!update) return;
        const action = updateAction(update);
        const run = action?.kind === 'install' ? this._run : IDLE;
        this._title.text = updateTitle(update);
        this._syncLine(update, run);
        this._syncAction(action, run);
        this._syncCommand(update, action);
    }

    _syncLine(update, run) {
        const line = runLine(run);
        this._detail.visible = line !== null;
        this._detail.text = line ?? '';
        setClass(this._detail, 'failed', run.phase === 'failed');
        this._whatsNewLabel.text = _("What's new");
        this._whatsNew.visible = line === null && showsWhatsNew(update);
        this._syncBadge(run.phase);
    }

    _syncBadge(phase) {
        if (phase === this._badgePhase) return;
        this._badgePhase = phase;
        this._badge.child?.destroy();
        const actor = badgeActor(this._ctx.motion, phase);
        this._badge.set_child(actor);
        this._badge.visible = actor !== null;
    }

    _syncAction(action, run) {
        const busy = run.phase === 'running' || run.phase === 'done';
        this._action.visible = action !== null && !busy;
        if (!action) return;
        const retry = run.phase === 'failed';
        this._actionLabel.text = retry ? _('Retry') : action.label;
        setClass(this._action, 'primary', action.kind === 'install' && !retry);
        setClass(this._action, 'checked', action.kind === 'command' && this._expanded);
        this._action.accessible_name = this._actionLabel.text;
    }

    _syncCommand(update, action) {
        this._command.visible = action?.kind === 'command' && this._expanded;
        this._commandText.text = update.command ?? '';
        this._copyLabel.text = this._copied ? _('Copied') : _('Copy');
    }

    _onAction() {
        const kind = updateAction(this._update)?.kind;
        if (kind === 'install') this._ctx.actions.installUpdate();
        else if (kind === 'notes') this._openRelease();
        else if (kind === 'command') this._toggleCommand();
    }

    _toggleCommand() {
        this._expanded = !this._expanded;
        this._copied = false;
        this._sync();
    }

    _copyCommand() {
        if (!this._update?.command) return;
        this._ctx.actions.copy(this._update.command);
        this._copied = true;
        this._sync();
    }

    _openRelease() {
        if (this._update?.url) this._ctx.actions.openUrl(this._update.url);
    }
}
