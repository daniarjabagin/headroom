import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';
import { _, fill } from '../i18n.js';
import { supports06 } from '../settings.js';
import { LoggingRows } from './loggingRows.js';
import { group } from './rows.js';
import { restartService, serviceActive } from './systemdUnit.js';
import { spinner } from './widgets.js';

function suffixButton(label, onClick) {
    const button = new Gtk.Button({ label, valign: Gtk.Align.CENTER });
    button.connect('clicked', () => onClick());
    return button;
}

function resetRow(onActivate) {
    const row = new Adw.ButtonRow({ title: _('Reset all settings…') });
    row.add_css_class('destructive-action');
    row.connect('activated', () => onActivate());
    return row;
}

export class AdvancedPage {
    constructor({ client, window, toast }) {
        this._client = client;
        this._window = window;
        this._toast = toast;
        this._cancellable = new Gio.Cancellable();
        this._diagnostics = null;
        this._diagnosticsAsked = false;
        this._systemd = false;
        this.page = new Adw.PreferencesPage({ title: _('Advanced'), icon_name: 'emblem-system-symbolic' });
        this._logging = new LoggingRows(client, toast);
        this._troubleshooting = this._troubleshootingGroup();
        this._reset = group('', [resetRow(() => this._confirmReset())]);
        for (const entry of [this._serviceGroup(), this._logging.group, this._troubleshooting, this._reset])
            this.page.add(entry);
        this._checkSystemd();
    }

    update(settings, state) {
        this._settings = settings;
        this._version = state.appVersion;
        this._syncService();
        const supported = supports06(state);
        for (const entry of [this._logging.group, this._troubleshooting, this._reset]) entry.visible = supported;
        if (!supported) return;
        this._logging.update(settings, this._diagnostics);
        if (!this._diagnosticsAsked) this._loadDiagnostics();
    }

    disconnected() {
        this._diagnostics = null;
        this._diagnosticsAsked = false;
    }

    destroy() {
        this._cancellable.cancel();
    }

    _serviceGroup() {
        this._service = new Adw.ActionRow({ title: _('Headroom service'), use_markup: false });
        this._restartSpinner = spinner();
        this._restartSpinner.valign = Gtk.Align.CENTER;
        this._restartSpinner.visible = false;
        this._restart = suffixButton(_('Restart'), () => this._restartService());
        this._service.add_suffix(this._restartSpinner);
        this._service.add_suffix(this._restart);
        return group(_('Service'), [this._service]);
    }

    _syncService() {
        const parts = [_('Running')];
        if (this._version) parts.push(fill(_('version {version}'), { version: this._version }));
        if (this._systemd) parts.push(_('systemd user service'));
        this._service.subtitle = parts.join(' · ');
        this._restart.visible = this._systemd;
    }

    async _checkSystemd() {
        this._systemd = await serviceActive(this._cancellable);
        if (this._cancellable.is_cancelled()) return;
        if (this._settings) this._syncService();
    }

    async _restartService() {
        this._restart.sensitive = false;
        this._restartSpinner.visible = true;
        try {
            await restartService(this._cancellable);
        } catch (error) {
            if (!this._cancellable.is_cancelled()) this._toast(error.message);
        }
        if (this._cancellable.is_cancelled()) return;
        this._restart.sensitive = true;
        this._restartSpinner.visible = false;
    }

    _troubleshootingGroup() {
        const row = new Adw.ActionRow({
            title: _('Copy diagnostics'),
            subtitle: _('Versions, desktop and account states. No tokens or emails.'),
        });
        row.add_suffix(suffixButton(_('Copy'), () => this._copyDiagnostics()));
        this._diagnosticsRow = row;
        return group(_('Troubleshooting'), [row], _('Paste the diagnostics into a bug report.'));
    }

    async _loadDiagnostics() {
        this._diagnosticsAsked = true;
        try {
            this._diagnostics = await this._client.getDiagnostics();
        } catch (error) {
            if (!this._cancellable.is_cancelled()) this._toast(error.message);
            return;
        }
        if (this._diagnostics && this._settings) this._logging.update(this._settings, this._diagnostics);
    }

    async _copyDiagnostics() {
        try {
            const report = await this._client.getDiagnostics();
            if (!report) return;
            this._diagnosticsRow.get_clipboard().set(report.text);
            this._toast(_('Diagnostics copied'));
        } catch (error) {
            this._toast(error.message);
        }
    }

    _confirmReset() {
        const dialog = new Adw.AlertDialog({
            heading: _('Reset all settings?'),
            body: _('Accounts stay signed in; appearance, notifications and hidden limits return to defaults.'),
        });
        dialog.add_response('cancel', _('Cancel'));
        dialog.add_response('reset', _('Reset'));
        dialog.set_response_appearance('reset', Adw.ResponseAppearance.DESTRUCTIVE);
        dialog.default_response = 'cancel';
        dialog.close_response = 'cancel';
        dialog.connect('response', (_dialog, response) => response === 'reset' && this._resetSettings());
        dialog.present(this._window);
    }

    async _resetSettings() {
        try {
            await this._client.resetSettings();
        } catch (error) {
            this._toast(error.message);
        }
    }
}
