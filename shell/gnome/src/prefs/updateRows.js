import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { updatesPatch } from '../settings.js';
import { IDLE, runLine, showsWhatsNew, updateAction, updateTitle } from '../update.js';
import { switchRow } from './rows.js';
import { spinner } from './widgets.js';

function suffixButton(onClick, cssClasses = []) {
    const button = new Gtk.Button({ valign: Gtk.Align.CENTER, css_classes: cssClasses });
    button.connect('clicked', () => onClick());
    return button;
}

export class UpdateRows {
    constructor(client, runner) {
        this._runner = runner;
        this._update = null;
        this._copied = false;
        this.check = switchRow({
            title: _('Check for updates'),
            subtitle: _('Once a day, asks GitHub for the latest release. Nothing else is sent.'),
            onChange: value => client.updateSettings(updatesPatch(value)),
        });
        this.release = new Adw.ActionRow({ visible: false });
        this._spinner = spinner();
        this._spinner.valign = Gtk.Align.CENTER;
        this._whatsNew = suffixButton(() => this._openRelease(), ['flat']);
        this._action = suffixButton(() => this._onAction());
        for (const widget of [this._spinner, this._whatsNew, this._action]) this.release.add_suffix(widget);
    }

    update(settings, update) {
        this.check.set(settings.updates.check);
        if (update?.command !== this._update?.command) this._copied = false;
        this._update = update;
        this.sync();
    }

    sync() {
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
