import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { updatesPatch } from '../settings.js';
import { IDLE, runLine, showsWhatsNew, updateAction, updateTitle } from '../update.js';
import { checkRow } from '../updateCheck.js';
import { switchRow } from './rows.js';
import { spinner } from './widgets.js';

function suffixButton(onClick, cssClasses = []) {
    const button = new Gtk.Button({ valign: Gtk.Align.CENTER, css_classes: cssClasses });
    button.connect('clicked', () => onClick());
    return button;
}

export class UpdateRows {
    constructor(client, runner) {
        this._client = client;
        this._runner = runner;
        this._update = null;
        this._copied = false;
        this._check = { enabled: false, appVersion: null, updateCheck: null, result: null, checking: false };
        this.check = switchRow({
            title: _('Check for updates'),
            subtitle: _('Once a day, asks GitHub for the latest release. Nothing else is sent.'),
            onChange: value => client.updateSettings(updatesPatch(value)),
        });
        this.release = new Adw.ActionRow({ visible: false, use_markup: false });
        this._spinner = spinner();
        this._spinner.valign = Gtk.Align.CENTER;
        this._whatsNew = suffixButton(() => this._openRelease(), ['flat']);
        this._action = suffixButton(() => this._onAction());
        for (const widget of [this._spinner, this._whatsNew, this._action]) this.release.add_suffix(widget);
        this.status = new Adw.ActionRow({ visible: false, use_markup: false });
        this._statusSpinner = spinner();
        this._statusSpinner.valign = Gtk.Align.CENTER;
        this._checkButton = suffixButton(() => this._checkNow());
        this.status.add_suffix(this._statusSpinner);
        this.status.add_suffix(this._checkButton);
    }

    update(settings, state) {
        this.check.set(settings.updates.check);
        if (state.update?.command !== this._update?.command) this._copied = false;
        this._update = state.update;
        Object.assign(this._check, {
            enabled: settings.updates.check,
            appVersion: state.appVersion,
            updateCheck: state.updateCheck,
        });
        this.sync();
    }

    _syncStatus() {
        const row = checkRow({ ...this._check, update: this._update, now: new Date() });
        this.status.visible = row.visible;
        if (!row.visible) return;
        this.status.title = row.title;
        this.status.subtitle = row.subtitle;
        this._statusSpinner.visible = row.busy;
        this._checkButton.label = row.action;
        this._checkButton.sensitive = !row.busy;
    }

    async _checkNow() {
        if (this._check.checking) return;
        this._check.checking = true;
        this._syncStatus();
        const result = await this._client.checkForUpdates();
        if (result === null) return;
        Object.assign(this._check, { checking: false, result });
        this._syncStatus();
    }

    sync() {
        this._syncStatus();
        const update = this._update;
        this.release.visible = update !== null;
        if (!update) return;
        const action = updateAction(update);
        const run = action?.kind === 'install' ? this._runner.run : IDLE;
        this.release.title = updateTitle(update);
        this.release.subtitle = runLine(run) ?? (action?.kind === 'command' ? update.command : '');
        this.release.subtitle_selectable = action?.kind === 'command';
        this._spinner.visible = run.phase === 'running';
        this._whatsNew.label = _("What's new");
        this._whatsNew.visible = showsWhatsNew(update) && run.phase === 'idle';
        this._syncAction(action, run);
    }

    _syncAction(action, run) {
        this._action.visible = action !== null && (run.phase === 'idle' || run.phase === 'failed');
        if (!action) return;
        const labels = { install: action.label, command: this._copied ? _('Copied') : _('Copy'), notes: action.label };
        this._action.label = run.phase === 'failed' ? _('Retry') : labels[action.kind];
        if (action.kind === 'install' && run.phase === 'idle') this._action.add_css_class('suggested-action');
        else this._action.remove_css_class('suggested-action');
    }

    _onAction() {
        const kind = updateAction(this._update)?.kind;
        if (kind === 'install') this._runner.start();
        else if (kind === 'command') this._copyCommand();
        else if (kind === 'notes') this._openRelease();
    }

    _copyCommand() {
        this.release.get_clipboard().set(this._update.command);
        this._copied = true;
        this.sync();
    }

    _openRelease() {
        if (this._update?.url)
            new Gtk.UriLauncher({ uri: this._update.url }).launch(this.release.get_root(), null, null);
    }
}
