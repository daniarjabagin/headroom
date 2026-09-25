import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { _, fill } from '../../i18n.js';
import { loadBytes, picturesFolder, shareFileName } from './files.js';
import { providerIconPainter } from './shareIcon.js';
import { shareModel, shareText } from './shareModel.js';
import { renderShare } from './shareRender.js';

const PALETTE_KEYS = {
    bg: 'bg',
    fg: 'fg',
    dim: 'dim',
    line: 'line',
    accent: 'accent',
    card: 'card',
    track: 'track',
    tick: 'tick',
    text: 'text',
    textSecondary: 'text-secondary',
    ok: 'ok',
    warn: 'warn',
    crit: 'crit',
    none: 'none',
};

function readPalette(actor) {
    const node = actor.get_theme_node();
    const entries = Object.entries(PALETTE_KEYS).map(([key, name]) => {
        const [found, color] = node.lookup_color(`-headroom-share-${name}`, false);
        return [key, found ? color : node.get_foreground_color()];
    });
    return Object.fromEntries(entries);
}

function fontFamily() {
    const name = St.Settings.get().font_name;
    return Pango.FontDescription.from_string(name || 'Sans').get_family() ?? 'Sans';
}

function ensureFolder(folder) {
    try {
        folder.make_directory_with_parents(null);
    } catch (error) {
        if (!(error instanceof GLib.Error) || !error.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.EXISTS)) throw error;
    }
}

function folderLabel(folder) {
    const parent = folder.get_parent();
    return parent ? `${parent.get_basename()}/${folder.get_basename()}` : folder.get_basename();
}

function writeAtomically(surface, file) {
    const temporary = Gio.File.new_for_path(`${file.get_path()}.part`);
    surface.writeToPNG(temporary.get_path());
    temporary.move(file, Gio.FileCopyFlags.OVERWRITE, null, null);
}

export class Sharer {
    constructor(ctx, paletteActor) {
        this._ctx = ctx;
        this._paletteActor = paletteActor;
    }

    _model(card) {
        return shareModel(card, { display: this._ctx.display, now: this._ctx.now(), hour12: this._ctx.hour12() });
    }

    canShare(card) {
        return this._model(card) !== null;
    }

    copyText(card) {
        const model = this._model(card);
        if (!model) return;
        this._ctx.actions.copy(shareText(model));
        this._ctx.toast(_('Copied as text'), null);
    }

    async shareImage(card) {
        const model = this._model(card);
        if (!model) return;
        try {
            const folder = await this._render(model);
            this._ctx.toast(_('Image copied'), fill(_('· saved to {folder}'), { folder: folderLabel(folder) }));
        } catch (error) {
            logError(error, 'Headroom could not share the image');
            this._ctx.toast(_("Couldn't share the image"), error.message, 'dialog-warning');
        }
    }

    async _render(model) {
        const icon = await providerIconPainter(this._ctx.dir, model.provider);
        const surface = renderShare(model, { palette: readPalette(this._paletteActor), family: fontFamily(), icon });
        const folder = picturesFolder();
        ensureFolder(folder);
        const file = folder.get_child(shareFileName(model.provider, this._ctx.now()));
        writeAtomically(surface, file);
        surface.finish();
        const bytes = await loadBytes(file);
        St.Clipboard.get_default().set_content(St.ClipboardType.CLIPBOARD, 'image/png', new GLib.Bytes(bytes));
        return folder;
    }
}
