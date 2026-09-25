import Cairo from 'cairo';
import Pango from 'gi://Pango';
import { _ } from '../../i18n.js';
import { drawText, fillPolygon, fillRoundedRect, setColor } from './canvas.js';

export const SHARE_WIDTH = 1200;
export const SHARE_HEIGHT = 630;

const PAD = 56;
const RIGHT = SHARE_WIDTH - PAD;
const TOP_RULE = 113;
const BOTTOM_RULE = 548;
const BODY_MIDDLE = (TOP_RULE + BOTTOM_RULE) / 2;
const MAX_ROWS = 3;
const ROW_PITCH = 134;
const CARD = { x: 620, width: 524, radius: 24, padding: 18 };
const ROW_INSET = 28;
const LEFT_METER = { y: 442, width: 500, height: 14, gap: 6 };
const MARK_HEIGHT = 32;
const MARK_TOP = 48;
const MARK_PIECES = [
    {
        accent: false,
        points: [
            [12, 29],
            [29, 24],
            [29, 83],
            [12, 88],
        ],
    },
    {
        accent: false,
        points: [
            [38, 13],
            [55, 8],
            [55, 84],
            [38, 89],
        ],
    },
    {
        accent: false,
        points: [
            [64, 62],
            [81, 57],
            [81, 74],
            [64, 79],
        ],
    },
    {
        accent: true,
        points: [
            [64, 28],
            [81, 23],
            [81, 44],
            [64, 49],
        ],
    },
];
const MARK_BOX = { x: 12, y: 8, height: 81, width: 69 };

function style(family, size, color, weight = Pango.Weight.NORMAL, spacing = 0) {
    return { family, size, color, weight, spacing };
}

function drawMark(cr, palette) {
    const scale = MARK_HEIGHT / MARK_BOX.height;
    for (const piece of MARK_PIECES) {
        const points = piece.points.map(([x, y]) => [
            PAD + (x - MARK_BOX.x) * scale,
            MARK_TOP + (y - MARK_BOX.y) * scale,
        ]);
        fillPolygon(cr, points, piece.accent ? palette.accent : palette.fg);
    }
    return PAD + MARK_BOX.width * scale;
}

function drawTop(cr, model, palette, family) {
    const markRight = drawMark(cr, palette);
    drawText(cr, 'headroom', style(family, 24, palette.fg, Pango.Weight.BOLD, -0.4), {
        x: markRight + 14,
        baseline: 70,
    });
    drawText(cr, 'by asteru studio', style(family, 8, palette.fg), { x: markRight + 15, baseline: 82 });
    drawText(cr, model.date, style(family, 20, palette.dim, Pango.Weight.NORMAL, 1.6), {
        x: RIGHT,
        baseline: 72,
        align: 'right',
    });
    fillRoundedRect(cr, { x: PAD, y: TOP_RULE, width: RIGHT - PAD, height: 2 }, 0, palette.line);
}

function drawFooter(cr, palette, family) {
    fillRoundedRect(cr, { x: PAD, y: BOTTOM_RULE, width: RIGHT - PAD, height: 2 }, 0, palette.line);
    fillPolygon(
        cr,
        [
            [PAD, 575],
            [PAD + 16, 570],
            [PAD + 16, 586],
            [PAD, 591],
        ],
        palette.accent
    );
    const footer = style(family, 20, palette.dim);
    drawText(cr, _("Know what's left."), footer, { x: PAD + 28, baseline: 588 });
    drawText(cr, 'headroom', footer, { x: RIGHT, baseline: 588, align: 'right' });
}

function segmentBoxes(x, width, count, gap) {
    const each = (width - gap * (count - 1)) / count;
    return Array.from({ length: count }, (_unused, index) => ({ x: x + index * (each + gap), width: each }));
}

function drawMeter(cr, segments, box, colors) {
    const parts = segmentBoxes(box.x, box.width, Math.max(1, segments.length), box.gap);
    segments.forEach((segment, index) => {
        const part = { ...parts[index], y: box.y, height: box.height };
        fillRoundedRect(cr, part, box.height / 2, colors.track);
        const fillWidth = segment.fraction > 0 ? Math.max(box.height, part.width * segment.fraction) : 0;
        fillRoundedRect(cr, { ...part, width: fillWidth }, box.height / 2, colors.fill(segment));
        if (segment.tick === null || !colors.tick) return;
        const tickX = part.x + part.width * segment.tick - 2;
        fillRoundedRect(cr, { x: tickX, y: box.y - 4, width: 4, height: box.height + 8 }, 2, colors.tick);
    });
}

function drawIdentity(cr, model, palette, family, icon) {
    const iconSize = 32;
    if (icon) icon(cr, { x: PAD, y: 204, size: iconSize }, palette.fg);
    const nameX = icon ? PAD + iconSize + 14 : PAD;
    const name = drawText(cr, model.providerName, style(family, 26, palette.fg, Pango.Weight.BOLD), {
        x: nameX,
        baseline: 230,
    });
    if (model.detail)
        drawText(cr, `· ${model.detail}`, style(family, 26, palette.dim), {
            x: nameX + name.width + 14,
            baseline: 230,
        });
}

function drawHeadline(cr, model, palette, family) {
    const headline = model.headline;
    const big = drawText(cr, headline.percent, style(family, 116, palette.fg, Pango.Weight.BOLD, -3.5), {
        x: PAD - 4,
        baseline: 360,
    });
    drawText(cr, headline.word, style(family, 44, palette.fg, Pango.Weight.SEMIBOLD, -0.4), {
        x: big.x + big.width + 10,
        baseline: 358,
    });
    drawText(cr, headline.sub, style(family, 24, palette.dim), { x: PAD, baseline: 408 });
    const colors = { track: palette.line, fill: () => palette.fg, tick: null };
    drawMeter(cr, headline.segments, { x: PAD, ...LEFT_METER }, colors);
}

function toneColor(palette, tone) {
    return palette[tone] ?? palette.ok;
}

function drawRow(cr, row, top, palette, family) {
    const left = CARD.x + ROW_INSET;
    const right = CARD.x + CARD.width - ROW_INSET;
    drawText(cr, row.label, style(family, 26, palette.text, Pango.Weight.SEMIBOLD), { x: left, baseline: top + 54 });
    const colors = { track: palette.track, fill: segment => toneColor(palette, segment.tone), tick: palette.tick };
    drawMeter(cr, row.segments, { x: left, y: top + 72, width: right - left, height: 10, gap: 4 }, colors);
    drawText(cr, row.reading, style(family, 24, palette.text), { x: left, baseline: top + 116 });
    drawText(cr, row.reset, style(family, 24, palette.textSecondary), {
        x: right,
        baseline: top + 116,
        align: 'right',
    });
}

function drawCard(cr, model, palette, family) {
    const rows = model.rows.slice(0, MAX_ROWS);
    const height = CARD.padding + rows.length * ROW_PITCH;
    const y = Math.round(BODY_MIDDLE - height / 2);
    fillRoundedRect(cr, { x: CARD.x, y, width: CARD.width, height }, CARD.radius, palette.card);
    rows.forEach((row, index) => drawRow(cr, row, y + index * ROW_PITCH, palette, family));
}

export function renderShare(model, { palette, family, icon = null }) {
    const surface = new Cairo.ImageSurface(Cairo.Format.ARGB32, SHARE_WIDTH, SHARE_HEIGHT);
    const cr = new Cairo.Context(surface);
    setColor(cr, palette.bg);
    cr.paint();
    drawTop(cr, model, palette, family);
    drawIdentity(cr, model, palette, family, icon);
    drawHeadline(cr, model, palette, family);
    drawCard(cr, model, palette, family);
    drawFooter(cr, palette, family);
    cr.$dispose();
    return surface;
}
