import {
    donutGeometry,
    donutSegments,
    MIN_FRACTION,
    sectorPath,
    START_ANGLE,
    visibleFractions,
} from '../src/donutGeometry.js';
import { check } from './check.js';

const SIZE = 104;
const GEOMETRY = donutGeometry(SIZE);

function rounded(values) {
    return values.map(value => Math.round(value * 1e6) / 1e6);
}

function sum(values) {
    return values.reduce((total, value) => total + value, 0);
}

function pointOn(arc, angle) {
    return [arc.center[0] + arc.radius * Math.cos(angle), arc.center[1] + arc.radius * Math.sin(angle)];
}

function distanceToRay(point, angle) {
    const x = point[0] - GEOMETRY.center;
    const y = point[1] - GEOMETRY.center;
    return Math.abs(x * Math.sin(angle) - y * Math.cos(angle));
}

function edgeDistances(segment) {
    const arcs = sectorPath(GEOMETRY, segment);
    const endEdge = [pointOn(arcs[2], arcs[2].to), pointOn(arcs[3], arcs[3].from)];
    const startEdge = [pointOn(arcs[5], arcs[5].to), pointOn(arcs[0], arcs[0].from)];
    return [
        ...endEdge.map(point => distanceToRay(point, segment.end)),
        ...startEdge.map(point => distanceToRay(point, segment.start)),
    ];
}

function ordered(arcs) {
    return arcs.every(arc => (arc.negative ? arc.to <= arc.from : arc.to >= arc.from));
}

function testFractions() {
    check('proportional', rounded(visibleFractions([300, 100])), [0.75, 0.25]);
    check('nothing spent', visibleFractions([0, 0]), [0, 0]);
    check('single provider', visibleFractions([5]), [1]);
    const tiny = visibleFractions([1_000_000, 1]);
    check('tiny slice raised to the minimum', rounded([tiny[1]]), rounded([MIN_FRACTION]));
    check('tiny slices sum to one', rounded([sum(tiny)]), [1]);
    const three = visibleFractions([1_000_000, 0, 3]);
    check('zero and tiny share the minimum', rounded([three[1], three[2]]), rounded([MIN_FRACTION, MIN_FRACTION]));
    check('three sum to one', rounded([sum(three)]), [1]);
}

function testSegments() {
    const [ring] = donutSegments([1]);
    check('single ring is a full annulus', sectorPath(GEOMETRY, ring).length, 2);
    check('hidden before the sweep', donutSegments([0.75, 0.25], 0), []);
    const half = donutSegments([0.75, 0.25], 0.5);
    check(
        'sweep reveals the leading slice',
        [half.length, rounded([half[0].end])],
        [1, rounded([START_ANGLE + Math.PI])]
    );
}

function testSectors() {
    const [wide, narrow] = donutSegments([0.75, 0.25]);
    const wideArcs = sectorPath(GEOMETRY, wide);
    check('wide corners use the corner radius', rounded([wideArcs[0].radius]), rounded([GEOMETRY.corner]));
    check('edges keep half the gap', rounded(edgeDistances(wide)), rounded(Array(4).fill(GEOMETRY.gap / 2)));
    check('neighbour edges match', rounded(edgeDistances(narrow)), rounded(Array(4).fill(GEOMETRY.gap / 2)));
    check('arcs never wrap', ordered(wideArcs) && ordered(sectorPath(GEOMETRY, narrow)), true);
    const [, dot] = donutSegments(visibleFractions([1_000_000, 1]));
    const dotArcs = sectorPath(GEOMETRY, dot);
    check('tiny slice stays visible', dotArcs !== null, true);
    check('tiny slice corners shrink', dotArcs[0].radius > 0 && dotArcs[0].radius < GEOMETRY.corner, true);
    check('tiny slice arcs never wrap', ordered(dotArcs), true);
    check('tiny slice keeps the gap', rounded(edgeDistances(dot)), rounded(Array(4).fill(GEOMETRY.gap / 2)));
    check('sliver too thin to draw', sectorPath(GEOMETRY, { index: 0, start: 0, end: 0.01, gap: true }), null);
}

export function testDonut() {
    testFractions();
    testSegments();
    testSectors();
}
