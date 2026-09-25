import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { loggingPatch } from '../settings.js';
import { comboRow, group } from './rows.js';

function iconButton(iconName, tooltip, onClick) {
    const button = new Gtk.Button({
        icon_name: iconName,
        tooltip_text: tooltip,
        valign: Gtk.Align.CENTER,
        css_classes: ['flat'],
    });
    button.connect('clicked', () => onClick());
    return button;
}

export function expandHome(path) {
    return path.startsWith('~/') ? GLib.build_filenamev([GLib.get_home_dir(), path.slice(2)]) : path;
}

export class LoggingRows {
    constructor(client, toast) {
        this._toast = toast;
        this._path = null;
        this._level = comboRow({
            title: _('Log level'),
            subtitle: _('Debug adds provider responses without tokens'),
            options: [
                { value: 'error', label: _('Errors') },
                { value: 'warn', label: _('Warnings') },
                { value: 'info', label: _('Info') },
                { value: 'debug', label: _('Debug') },
            ],
            onChange: value => client.updateSettings(loggingPatch(value)),
        });
        this._file = new Adw.ActionRow({ title: _('Log file'), use_markup: false, subtitle_selectable: true });
        this._copy = iconButton('edit-copy-symbolic', _('Copy path'), () => this._copyPath());
        this._open = iconButton('folder-open-symbolic', _('Open folder'), () => this._openFolder());
        this._file.add_suffix(this._copy);
        this._file.add_suffix(this._open);
        this.group = group(_('Logging'), [this._level.row, this._file]);
    }

    update(settings, diagnostics) {
        this._level.set(settings.logging.level);
        const fromEnv = diagnostics?.logLevelSource === 'env';
        this._level.row.sensitive = !fromEnv;
        this._level.row.subtitle = fromEnv
            ? _('RUST_LOG is set and wins over this setting')
            : _('Debug adds provider responses without tokens');
        this._path = diagnostics?.logFile ?? null;
        this._file.subtitle = this._path ?? (diagnostics ? _('The log file could not be opened') : '');
        this._copy.sensitive = this._path !== null;
        this._open.sensitive = this._path !== null;
    }

    _copyPath() {
        if (!this._path) return;
        this._file.get_clipboard().set(this._path);
        this._toast(_('Path copied'));
    }

    _openFolder() {
        if (!this._path) return;
        const launcher = new Gtk.FileLauncher({ file: Gio.File.new_for_path(expandHome(this._path)) });
        launcher.open_containing_folder(this._file.get_root(), null, (source, result) => {
            try {
                source.open_containing_folder_finish(result);
            } catch (error) {
                this._toast(error.message);
            }
        });
    }
}
