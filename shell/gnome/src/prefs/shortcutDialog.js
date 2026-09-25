import Adw from 'gi://Adw';
import Gdk from 'gi://Gdk';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { keyboardArt, shortcutLabel } from './keycaps.js';
import { captureOutcome } from './shortcutModel.js';

const DIALOG_WIDTH = 400;

function label(text, cssClasses) {
    return new Gtk.Label({
        label: text,
        css_classes: cssClasses,
        wrap: true,
        justify: Gtk.Justification.CENTER,
        max_width_chars: 36,
    });
}

function currentLine(accelerator) {
    const line = new Gtk.Box({ spacing: 10, halign: Gtk.Align.CENTER, margin_top: 10 });
    line.append(label(_('Current'), ['dim-label', 'caption']));
    line.append(shortcutLabel(accelerator));
    return line;
}

function body(accelerator) {
    const box = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 4 });
    for (const side of ['start', 'end', 'bottom']) box[`margin_${side}`] = 32;
    box.margin_top = 6;
    const art = keyboardArt();
    art.margin_bottom = 18;
    box.append(art);
    box.append(label(_('Open Headroom'), ['dim-label']));
    box.append(label(_('Press your keyboard shortcut…'), ['title-2']));
    const hint = label(_('Press Esc to cancel or Backspace to disable the keyboard shortcut.'), ['dim-label']);
    hint.margin_top = 6;
    box.append(hint);
    box.append(currentLine(accelerator));
    return box;
}

export class ShortcutDialog {
    constructor({ accelerator, onSet }) {
        this._onSet = onSet;
        const toolbar = new Adw.ToolbarView({ content: body(accelerator) });
        toolbar.add_top_bar(new Adw.HeaderBar());
        this.dialog = new Adw.Dialog({ title: _('Set Shortcut'), content_width: DIALOG_WIDTH, child: toolbar });
        const keys = new Gtk.EventControllerKey({ propagation_phase: Gtk.PropagationPhase.CAPTURE });
        keys.connect('key-pressed', (_controller, keyval, _keycode, state) => this._onKey(keyval, state));
        this.dialog.add_controller(keys);
    }

    present(parent) {
        this.dialog.present(parent);
    }

    _onKey(keyval, state) {
        const modifiers = state & Gtk.accelerator_get_default_mod_mask();
        const outcome = captureOutcome({
            keyName: Gdk.keyval_name(keyval),
            hasModifiers: modifiers !== 0,
            accelerator: Gtk.accelerator_valid(keyval, modifiers) ? Gtk.accelerator_name(keyval, modifiers) : null,
        });
        if (outcome.kind === 'wait') return Gdk.EVENT_STOP;
        if (outcome.kind === 'disable') this._onSet('');
        if (outcome.kind === 'set') this._onSet(outcome.accelerator);
        this.dialog.close();
        return Gdk.EVENT_STOP;
    }
}
