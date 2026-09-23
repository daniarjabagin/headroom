import Gio from 'gi://Gio';
import { GENERIC_ICON, iconCandidates } from './providers.js';

export function providerIconFile(dir, id) {
    const icons = dir.get_child('icons');
    for (const candidate of iconCandidates(id)) {
        const file = icons.get_child(candidate.file);
        if (file.query_exists(null)) return { gicon: new Gio.FileIcon({ file }), tinted: candidate.tinted };
    }
    return { gicon: new Gio.FileIcon({ file: icons.get_child(GENERIC_ICON.file) }), tinted: GENERIC_ICON.tinted };
}
