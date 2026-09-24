import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _, fill } from '../i18n.js';
import { ProgressProcess } from '../cli.js';
import { flowBody, navigationPage, pageStack, resultPage, stack } from './flowPage.js';
import { addAccountArgs, LABEL_MAX_CHARS } from './registry.js';
import { pillButton, spinner } from './widgets.js';

export class ApiKeyPage {
    constructor({ dir, provider, method, onClose }) {
        this._provider = provider;
        this._method = method;
        this._onClose = onClose;
        this._process = null;
        const { box, description } = flowBody(
            dir,
            provider.id,
            fill(_('Connect {provider}'), { provider: provider.displayName })
        );
        this._description = description;
        this._stack = pageStack();
        this._stack.add_named(this._formPage(), 'form');
        this._stack.add_named(this._progressPage(), 'progress');
        this._stack.add_named(
            resultPage('done', _('Done'), () => this._onClose()),
            'done'
        );
        this._stack.add_named(
            resultPage('error', _('Try Again'), () => this._showForm()),
            'error'
        );
        box.append(this._stack);
        this.page = navigationPage(fill(_('Add {provider} Account'), { provider: provider.displayName }), box);
        this._showForm();
    }

    cancel() {
        this._process?.cancel();
        this._process = null;
    }

    _formPage() {
        this._keyRow = new Adw.PasswordEntryRow({ title: this._method.label ?? _('API key') });
        this._keyRow.connect('changed', () => this._syncAddButton());
        this._keyRow.connect('entry-activated', () => this._start());
        this._labelRow = new Adw.EntryRow({ title: _('Label (optional)'), max_length: LABEL_MAX_CHARS });
        this._labelRow.connect('entry-activated', () => this._start());
        const group = new Adw.PreferencesGroup({ description: this._method.hint ?? '' });
        group.add(this._keyRow);
        group.add(this._labelRow);
        this._addButton = pillButton(_('Add'), true, () => this._start());
        const buttons = new Gtk.Box({ spacing: 12, halign: Gtk.Align.CENTER });
        if (this._method.consoleUrl) buttons.append(pillButton(_('Get a Key'), false, () => this._openConsole()));
        buttons.append(this._addButton);
        return stack([group, buttons]);
    }

    _progressPage() {
        const status = new Gtk.Box({ spacing: 10, halign: Gtk.Align.CENTER });
        status.append(spinner());
        status.append(new Gtk.Label({ label: _('Checking the key…'), css_classes: ['heading'] }));
        return stack([status, pillButton(_('Cancel'), false, () => this._onClose())]);
    }

    _syncAddButton() {
        this._addButton.sensitive = this._keyRow.text.trim().length > 0;
    }

    _showForm() {
        this._stack.visible_child_name = 'form';
        this._description.label = fill(
            _('Headroom checks the key with {provider} and keeps it in your keyring. It never leaves this computer.'),
            { provider: this._provider.displayName }
        );
        this._syncAddButton();
    }

    _start() {
        const key = this._keyRow.text.trim();
        if (!key || this._process) return;
        this._stack.visible_child_name = 'progress';
        this._description.label = _('This takes a moment.');
        const args = addAccountArgs(this._provider.id, this._method, this._labelRow.text);
        try {
            this._process = new ProgressProcess(args, {
                onEvent: event => this._onEvent(event),
                onExit: error => this._onExit(error),
            });
        } catch (error) {
            this._fail(error.message);
            return;
        }
        this._process.write(key);
        this._process.closeInput();
    }

    _onEvent(event) {
        if (event.event === 'done') this._succeed();
        else if (event.event === 'error') this._fail(event.message);
    }

    _onExit(error) {
        this._process = null;
        if (this._stack.visible_child_name !== 'progress') return;
        if (error) this._fail(error);
        else this._succeed();
    }

    _succeed() {
        this._keyRow.text = '';
        this._stack.visible_child_name = 'done';
        this._description.label = _('Account added. It shows up in the panel in a moment.');
    }

    _fail(message) {
        this.cancel();
        this._stack.visible_child_name = 'error';
        this._description.label = message;
    }

    _openConsole() {
        new Gtk.UriLauncher({ uri: this._method.consoleUrl }).launch(this.page.get_root(), null, null);
    }
}
