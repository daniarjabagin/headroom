const HOLE_RATIO = 0.618;
const GAP_RATIO = 2 / 104;
const CORNER_RATIO = 0.2;
const MIN_WIDTH_RATIO = 3 / 104;
const FULL_TURN = 2 * Math.PI;
const QUARTER_TURN = Math.PI / 2;
const ANGLE_EPSILON = 1e-9;
export const START_ANGLE = -QUARTER_TURN;

export function donutGeometry(size) {
    const outer = size / 2;
    const thickness = outer * (1 - HOLE_RATIO);
    return {
        center: outer,
        outer,
        inner: outer - thickness,
        thickness,
        gap: GAP_RATIO * size,
        corner: CORNER_RATIO * thickness,
    };
}

const UNIT = donutGeometry(1);
export const MIN_FRACTION = (MIN_WIDTH_RATIO + UNIT.gap) / UNIT.inner / FULL_TURN;

function minimumFraction(count) {
    return count > 1 ? Math.min(1 / count, MIN_FRACTION) : 0;
}

function shares(values, raised, minimum) {
    const free = values.map((value, index) => (raised.has(index) ? 0 : value));
    const freeTotal = free.reduce((sum, value) => sum + value, 0);
    const share = 1 - raised.size * minimum;
    return values.map((_value, index) => (raised.has(index) ? minimum : (free[index] / freeTotal) * share));
}

function spread(values, minimum) {
    const raised = new Set();
    const tooSmall = fractions => fractions.findIndex((fraction, index) => !raised.has(index) && fraction < minimum);
    let fractions = shares(values, raised, minimum);
    for (let index = tooSmall(fractions); index >= 0; index = tooSmall(fractions)) {
        raised.add(index);
        fractions = shares(values, raised, minimum);
    }
    return fractions;
}

export function visibleFractions(values) {
    const total = values.reduce((sum, value) => sum + value, 0);
    if (total <= 0) return values.map(() => 0);
    return spread(values, minimumFraction(values.length));
}

export function donutSegments(fractions, reveal = 1) {
    if (reveal <= 0) return [];
    if (fractions.length === 1) {
        const end = START_ANGLE + Math.min(reveal, fractions[0]) * FULL_TURN;
        return end > START_ANGLE ? [{ index: 0, start: START_ANGLE, end, gap: false }] : [];
    }
    const limit = START_ANGLE + reveal * FULL_TURN;
    const segments = [];
    let angle = START_ANGLE;
    fractions.forEach((fraction, index) => {
        const start = angle;
        angle += fraction * FULL_TURN;
        const end = Math.min(angle, limit);
        if (end > start) segments.push({ index, start, end, gap: true });
    });
    return segments;
}

function cornerRadius(geometry, halfSweep, halfGap) {
    const sin = Math.sin(Math.min(halfSweep, QUARTER_TURN));
    const innerRoom = (geometry.inner * sin - halfGap) / (1 - sin);
    const outerRoom = (geometry.outer * sin - halfGap) / (1 + sin);
    const room = Math.min(innerRoom, outerRoom);
    return room < 0 ? null : Math.min(geometry.corner, room);
}

function polar(geometry, radius, angle) {
    return [geometry.center + radius * Math.cos(angle), geometry.center + radius * Math.sin(angle)];
}

function ringPath(geometry) {
    const center = [geometry.center, geometry.center];
    return [
        { center, radius: geometry.outer, from: 0, to: FULL_TURN },
        { center, radius: geometry.inner, from: FULL_TURN, to: 0, negative: true, newSubPath: true },
    ];
}

function sectorArcs(geometry, { start, end }, corner, outerOffset, innerOffset) {
    const center = [geometry.center, geometry.center];
    const outerCorner = angle => polar(geometry, geometry.outer - corner, angle);
    const innerCorner = angle => polar(geometry, geometry.inner + corner, angle);
    const fillet = (at, from, to) => ({ center: at, radius: corner, from, to });
    return [
        fillet(outerCorner(start + outerOffset), start - QUARTER_TURN, start + outerOffset),
        {
            center,
            radius: geometry.outer,
            from: start + outerOffset,
            to: Math.max(end - outerOffset, start + outerOffset),
        },
        fillet(outerCorner(end - outerOffset), end - outerOffset, end + QUARTER_TURN),
        fillet(innerCorner(end - innerOffset), end + QUARTER_TURN, end - innerOffset + Math.PI),
        {
            center,
            radius: geometry.inner,
            from: end - innerOffset,
            to: Math.min(start + innerOffset, end - innerOffset),
            negative: true,
        },
        fillet(innerCorner(start + innerOffset), start + innerOffset + Math.PI, start + 3 * QUARTER_TURN),
    ];
}

export function sectorPath(geometry, segment) {
    if (!segment.gap && segment.end - segment.start >= FULL_TURN - ANGLE_EPSILON) return ringPath(geometry);
    const halfGap = segment.gap ? geometry.gap / 2 : 0;
    const corner = cornerRadius(geometry, (segment.end - segment.start) / 2, halfGap);
    if (corner === null) return null;
    const outerOffset = Math.asin((halfGap + corner) / (geometry.outer - corner));
    const innerOffset = Math.asin((halfGap + corner) / (geometry.inner + corner));
    return sectorArcs(geometry, segment, corner, outerOffset, innerOffset);
}
