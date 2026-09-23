import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import Pango from 'gi://Pango';
import { _, fill } from '../i18n.js';
import { providerInfo } from '../providers.js';
import { ProgressProcess } from './cli.js';
import { LogView } from './logView.js';
import { providerImage, pillButton, spinner, wrapLabel } from './widgets.js';

function page(children) {
    const box = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 12 });
    for (const child of children) box.append(child);
    return box;
}

export class AddAccountDialog {
    constructor({ provider, dir }) {
        this._provider = provider;
        this._info = providerInfo(provider);
        this._process = null;
        this.dialog = new Adw.Dialog({ title: fill(_('Add {provider} Account'), { provider: this._info.name }) });
        this.dialog.content_width = 460;
        this.dialog.connect('closed', () => this._process?.cancel());
        const toolbar = new Adw.ToolbarView();
        toolbar.add_top_bar(new Adw.HeaderBar());
        toolbar.content = this._body(dir);
        this.dialog.child = toolbar;
        this._showForm();
    }

    present(parent) {
        this.dialog.present(parent);
    }

    _body(dir) {
        const body = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 18 });
        for (const side of ['top', 'bottom', 'start', 'end']) body[`margin_${side}`] = 24;
        body.margin_top = 6;
        const logo = providerImage(dir, this._provider, 48);
        const title = new Gtk.Label({ label: fill(_('Sign in to {provider}'), { provider: this._info.name }) });
        title.add_css_class('title-2');
        this._description = wrapLabel('', ['dim-label']);
        this._description.justify = Gtk.Justification.CENTER;
        this._stack = new Gtk.Stack({ transition_type: Gtk.StackTransitionType.CROSSFADE, vhomogeneous: false });
        this._stack.add_named(this._formPage(), 'form');
        this._stack.add_named(this._progressPage(), 'progress');
        this._stack.add_named(this._resultPage('done'), 'done');
        this._stack.add_named(this._resultPage('error'), 'error');
        this._log = new LogView();
        for (const child of [logo, title, this._description, this._stack, this._log.widget]) body.append(child);
        return body;
    }

    _formPage() {
        this._labelEntry = new Adw.EntryRow({ title: _('Label (optional)') });
        this._labelEntry.connect('entry-activated', () => this._start());
        const group = new Adw.PreferencesGroup();
        group.add(this._labelEntry);
        return page([group, pillButton(_('Continue'), true, () => this._start())]);
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
        const cancel = pillButton(_('Cancel'), false, () => this.dialog.close());
        return page([status, this._openButton, this._codeGroup, cancel]);
    }

    _resultPage(kind) {
        const icon = new Gtk.Image({
            icon_name: kind === 'done' ? 'object-select-symbolic' : 'dialog-warning-symbolic',
            pixel_size: 32,
            css_classes: [kind === 'done' ? 'success' : 'warning'],
        });
        const action =
            kind === 'done'
                ? pillButton(_('Done'), true, () => this.dialog.close())
                : pillButton(_('Try Again'), true, () => this._showForm());
        return page([icon, action]);
    }

    _showForm() {
        this._stack.visible_child_name = 'form';
        this._description.label = fill(
            _(
                'Headroom signs in with the {provider} CLI in its own folder, so the account you use today stays signed in.'
            ),
            { provider: this._info.name }
        );
        this._log.clear();
    }

    _start() {
        const label = this._labelEntry.text.trim();
        const args = ['accounts', 'add', this._provider, ...(label ? ['--label', label] : [])];
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
        this._process?.cancel();
        this._process = null;
        this._stack.visible_child_name = 'error';
        this._description.label = message;
    }

    _openUrl() {
        if (this._url) new Gtk.UriLauncher({ uri: this._url }).launch(this.dialog.get_root(), null, null);
    }

    _sendCode() {
        const code = this._codeEntry.text.trim();
        if (!code || !this._process) return;
        this._process.write(code);
        this._log.append(_('Code sent'));
        this._codeEntry.text = '';
    }
}
