.pragma library

const CORNER_RATIO = 0.15;
const FULL_TURN = 360;

function geometry(size, holeRatio, gap) {
    const outer = size / 2;
    const inner = outer * holeRatio;
    return {
        center: outer,
        outer,
        inner,
        gap,
        corner: (outer - inner) * CORNER_RATIO
    };
}

function radians(degrees) {
    return degrees * Math.PI / 180;
}

function degrees(radiansValue) {
    return radiansValue * 180 / Math.PI;
}

function edgeInset(geo, radius) {
    return Math.asin(Math.min(1, geo.gap / 2 / radius));
}

function minSweep(geo) {
    return degrees(2 * edgeInset(geo, geo.inner) + geo.corner / geo.inner);
}

function innerLength(geo, sweep) {
    return geo.inner * (radians(sweep) - 2 * edgeInset(geo, geo.inner));
}

function cornerRadius(geo, sweep) {
    return Math.max(0, Math.min(geo.corner, innerLength(geo, sweep) / 2));
}

function point(geo, angle, radius) {
    return {
        x: geo.center + radius * Math.cos(angle),
        y: geo.center + radius * Math.sin(angle)
    };
}

function edgeFrame(angle, side) {
    return {
        along: {
            x: Math.cos(angle),
            y: Math.sin(angle)
        },
        across: {
            x: -Math.sin(angle) * side,
            y: Math.cos(angle) * side
        }
    };
}

function corner(geo, edgeAngle, side, radius, bulge) {
    const offset = geo.gap / 2;
    const reach = radius + bulge;
    const centerAngle = edgeAngle + side * Math.asin(Math.min(1, (offset + Math.abs(bulge)) / reach));
    const frame = edgeFrame(edgeAngle, side);
    const along = reach * Math.cos(centerAngle - edgeAngle);
    return {
        onArc: point(geo, centerAngle, radius),
        onEdge: {
            x: geo.center + frame.along.x * along + frame.across.x * offset,
            y: geo.center + frame.along.y * along + frame.across.y * offset
        },
        centerAngle
    };
}

function corners(geo, start, sweep) {
    const c = cornerRadius(geo, sweep);
    const a0 = radians(start);
    const a1 = radians(start + sweep);
    return {
        radius: c,
        outerStart: corner(geo, a0, 1, geo.outer, -c),
        outerEnd: corner(geo, a1, -1, geo.outer, -c),
        innerEnd: corner(geo, a1, -1, geo.inner, c),
        innerStart: corner(geo, a0, 1, geo.inner, c)
    };
}

function fmt(p) {
    return `${p.x.toFixed(3)} ${p.y.toFixed(3)}`;
}

function arc(radius, large, sweepFlag, to) {
    const r = radius.toFixed(3);
    return `A ${r} ${r} 0 ${large ? 1 : 0} ${sweepFlag} ${fmt(to)}`;
}

function isLarge(from, to) {
    return to.centerAngle - from.centerAngle > Math.PI;
}

function sectorPath(geo, start, sweep) {
    if (innerLength(geo, sweep) <= 0)
        return "";
    const k = corners(geo, start, sweep);
    return [
        `M ${fmt(k.outerStart.onArc)}`,
        arc(geo.outer, isLarge(k.outerStart, k.outerEnd), 1, k.outerEnd.onArc),
        arc(k.radius, false, 1, k.outerEnd.onEdge),
        `L ${fmt(k.innerEnd.onEdge)}`,
        arc(k.radius, false, 1, k.innerEnd.onArc),
        arc(geo.inner, isLarge(k.innerStart, k.innerEnd), 0, k.innerStart.onArc),
        arc(k.radius, false, 1, k.innerStart.onEdge),
        `L ${fmt(k.outerStart.onEdge)}`,
        arc(k.radius, false, 1, k.outerStart.onArc),
        "Z"
    ].join(" ");
}

function circlePath(geo, radius) {
    const top = point(geo, -Math.PI / 2, radius);
    const bottom = point(geo, Math.PI / 2, radius);
    return `M ${fmt(top)} ${arc(radius, false, 1, bottom)} ${arc(radius, false, 1, top)} Z`;
}

function ringPath(geo) {
    return `${circlePath(geo, geo.outer)} ${circlePath(geo, geo.inner)}`;
}

function slicePath(geo, start, sweep) {
    return sweep >= FULL_TURN ? ringPath(geo) : sectorPath(geo, start, sweep);
}
