import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { providerInfo } from './providers.js';

export function column(params = {}) {
    const box = new St.BoxLayout(params);
    box.layout_manager.orientation = Clutter.Orientation.VERTICAL;
    return box;
}

export function row(params = {}) {
    return new St.BoxLayout(params);
}

export function label(text, styleClass, params = {}) {
    return new St.Label({ text, style_class: styleClass, y_align: Clutter.ActorAlign.CENTER, ...params });
}

export function wrappingLabel(text, styleClass, params = {}) {
    const widget = label(text, styleClass, { x_expand: true, ...params });
    widget.clutter_text.line_wrap = true;
    widget.clutter_text.line_wrap_mode = Pango.WrapMode.WORD_CHAR;
    widget.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
    return widget;
}

export function spacer() {
    return new St.Widget({ x_expand: true });
}

export function themeIcon(iconName, styleClass) {
    return new St.Icon({ icon_name: iconName, style_class: styleClass, y_align: Clutter.ActorAlign.CENTER });
}

export function fileIcon(dir, fileName, styleClass) {
    const gicon = new Gio.FileIcon({ file: dir.get_child('icons').get_child(fileName) });
    return new St.Icon({ gicon, style_class: styleClass, y_align: Clutter.ActorAlign.CENTER });
}

export function providerIcon(dir, provider, styleClass) {
    const info = providerInfo(provider);
    const classes = `${styleClass}${info.tinted ? ' tinted' : ''}`;
    if (info.icon === null) return themeIcon('application-x-executable-symbolic', classes);
    return fileIcon(dir, info.icon, classes);
}

export function button(child, styleClass, onClick) {
    const widget = new St.Button({
        child,
        style_class: styleClass,
        reactive: true,
        can_focus: true,
        track_hover: true,
        y_align: Clutter.ActorAlign.CENTER,
    });
    widget.connect('clicked', () => onClick());
    return widget;
}

export function textButton(text, styleClass, onClick) {
    return button(new St.Label({ text }), styleClass, onClick);
}
