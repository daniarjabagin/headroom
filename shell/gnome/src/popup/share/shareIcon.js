import { providerIconFile } from '../../providerIcons.js';
import { loadBytes } from './files.js';

const SYMBOLIC_FILL = /#bebebe/gi;

async function loadRsvg() {
    try {
        return (await import('gi://Rsvg?version=2.0')).default;
    } catch {
        return null;
    }
}

function cssColor(color) {
    return `rgb(${color.red}, ${color.green}, ${color.blue})`;
}

export async function providerIconPainter(dir, provider) {
    const Rsvg = await loadRsvg();
    if (!Rsvg) return null;
    const { gicon, tinted } = providerIconFile(dir, provider);
    const source = new TextDecoder().decode(await loadBytes(gicon.get_file()));
    return (cr, box, color) => {
        const svg = tinted ? source.replace(SYMBOLIC_FILL, cssColor(color)) : source;
        const handle = Rsvg.Handle.new_from_data(new TextEncoder().encode(svg));
        const viewport = new Rsvg.Rectangle({ x: box.x, y: box.y, width: box.size, height: box.size });
        handle.render_document(cr, viewport);
    };
}
