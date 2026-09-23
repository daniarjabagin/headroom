import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import Pango from 'gi://Pango';
import { _, fill } from '../i18n.js';
import { ProgressProcess } from './cli.js';
import { flowBody, navigationPage, pageStack, resultPage, stack } from './flowPage.js';
import { LogView } from './logView.js';
import { addAccountArgs, LABEL_MAX_CHARS } from './registry.js';
import { pillButton, spinner } from './widgets.js';

export class CliLoginPage {
    constructor({ dir, provider, method, onClose }) {
        this._provider = provider;
        this._method = method;
        this._onClose = onClose;
        this._process = null;
        const { box, description } = flowBody(
            dir,
            provider.id,
            fill(_('Sign in to {provider}'), { provider: provider.displayName })
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
        this._log = new LogView();
        box.append(this._stack);
        box.append(this._log.widget);
        this.page = navigationPage(fill(_('Add {provider} Account'), { provider: provider.displayName }), box);
        this._showForm();
    }

    cancel() {
        this._process?.cancel();
        this._process = null;
    }

    _formPage() {
        this._labelEntry = new Adw.EntryRow({ title: _('Label (optional)'), max_length: LABEL_MAX_CHARS });
        this._labelEntry.connect('entry-activated', () => this._start());
        const group = new Adw.PreferencesGroup();
        group.add(this._labelEntry);
        return stack([group, pillButton(_('Continue'), true, () => this._start())]);
    }

    _progressPage() {
        this._status = new Gtk.Label({ css_classes: ['heading'], ellipsize: Pango.EllipsizeMode.END });
        const status = new Gtk.Box({ spacing: 10, halign: Gtk.Align.CENTER });
        status.append(spinner());
        status.append(this._status);
        this._openButton = pillButton(_('Open Sign-In Page'), true, () => this._openUrl());
        this._codeEntry = new Adw.EntryRow({ title: _('Paste code'), show_apply_button: true });
        this._codeEntry.connect('apply', () => this._sendCode());
        this._codeGroup = new Adw.PreferencesGroup({ description: _('Only if the sign-in page shows a code.') });
        this._codeGroup.add(this._codeEntry);
        const cancel = pillButton(_('Cancel'), false, () => this._onClose());
        return stack([status, this._openButton, this._codeGroup, cancel]);
    }

    _showForm() {
        this._stack.visible_child_name = 'form';
        this._description.label = fill(
            _(
                'Headroom signs in with the {provider} CLI in its own folder, so the account you use today stays signed in.'
            ),
            { provider: this._provider.displayName }
        );
        this._log.clear();
    }

    _start() {
        const args = addAccountArgs(this._provider.id, this._method, this._labelEntry.text);
        this._url = null;
        this._openButton.visible = false;
        this._codeGroup.visible = false;
        this._status.label = _('Starting sign-in…');
        this._description.label = _('This takes a moment.');
        this._stack.visible_child_name = 'progress';
        try {
            this._process = new ProgressProcess(args, {
                onEvent: event => this._onEvent(event),
                onExit: error => this._onExit(error),
            });
        } catch (error) {
            this._fail(error.message);
        }
    }

    _onEvent(event) {
        if (event.event === 'url') this._onUrl(event.url);
        else if (event.event === 'output') this._log.append(event.line);
        else if (event.event === 'done') this._succeed();
        else if (event.event === 'error') this._fail(event.message);
    }

    _onUrl(url) {
        this._url = url;
        this._log.append(url);
        this._status.label = _('Waiting for sign-in…');
        this._description.label = _('Open the sign-in page and finish in your browser.');
        this._openButton.visible = true;
        this._codeGroup.visible = true;
    }

    _onExit(error) {
        this._process = null;
        if (this._stack.visible_child_name !== 'progress') return;
        if (error) this._fail(error);
        else this._succeed();
    }

    _succeed() {
        this._stack.visible_child_name = 'done';
        this._description.label = _('Account added. It shows up in the panel in a moment.');
    }

    _fail(message) {
        this.cancel();
        this._stack.visible_child_name = 'error';
        this._description.label = message;
    }

    _openUrl() {
        if (this._url) new Gtk.UriLauncher({ uri: this._url }).launch(this.page.get_root(), null, null);
    }

    _sendCode() {
        const code = this._codeEntry.text.trim();
        if (!code || !this._process) return;
        this._process.write(code);
        this._log.append(_('Code sent'));
        this._codeEntry.text = '';
    }
}
