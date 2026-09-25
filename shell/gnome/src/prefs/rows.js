import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { OptionModel } from './optionModel.js';

function guarded(apply) {
    const state = { syncing: false };
    return {
        sync(update) {
            state.syncing = true;
            update();
            state.syncing = false;
        },
        emit(value) {
            if (!state.syncing) apply(value);
        },
    };
}

function toggleGroupRow({ title, subtitle, options, onChange }) {
    const row = new Adw.ActionRow({ title, subtitle: subtitle ?? '' });
    const group = new Adw.ToggleGroup({ valign: Gtk.Align.CENTER });
    for (const option of options) group.add(new Adw.Toggle({ name: option.value, label: option.label }));
    row.add_suffix(group);
    const guard = guarded(onChange);
    group.connect('notify::active-name', () => guard.emit(group.active_name));
    return { row, set: value => guard.sync(() => (group.active_name = value)) };
}

export function comboRow({ title, subtitle, options, onChange }) {
    const row = new Adw.ComboRow({ title, subtitle: subtitle ?? '' });
    const guard = guarded(onChange);
    const model = new OptionModel();
    const select = value => {
        const index = model.indexOf(value);
        if (row.selected !== index) guard.sync(() => (row.selected = index));
    };
    const setOptions = entries => {
        const current = model.valueAt(row.selected);
        if (!model.setOptions(entries)) return;
        guard.sync(() => (row.model = Gtk.StringList.new(model.labels)));
        select(current);
    };
    setOptions(options);
    row.connect('notify::selected', () => {
        const value = model.valueAt(row.selected);
        if (value !== null) guard.emit(value);
    });
    return { row, setOptions, set: select };
}

export function segmentedRow(params) {
    return Adw.ToggleGroup ? toggleGroupRow(params) : comboRow(params);
}

export function switchRow({ title, subtitle, onChange }) {
    const row = new Adw.SwitchRow({ title, subtitle: subtitle ?? '' });
    const guard = guarded(onChange);
    row.connect('notify::active', () => guard.emit(row.active));
    return { row, set: value => guard.sync(() => (row.active = value)) };
}

export function group(title, rows, description = '') {
    const actor = new Adw.PreferencesGroup({ title, description });
    for (const row of rows) actor.add(row);
    return actor;
}
