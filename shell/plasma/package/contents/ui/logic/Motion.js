.pragma library

const MAX_STEP = 0.12;
const SPREAD = 0.45;
const OPEN_FACTOR = 3;
const SHIMMER_FACTOR = 7;
const SPIN_FACTOR = 7;
const TURN = 360;
const SHEEN_SWEEP_MS = 1600;
const SHEEN_REST_MS = 3500;
const SHEEN_MIN_FILL = 0.03;
const SHEEN_LIGHTEN = 1.45;
const SHEEN_PEAK = 0.55;

function enabled(units, reduced) {
    return units.longDuration > 0 && reduced !== true;
}

function clamp01(value) {
    return Math.min(1, Math.max(0, value));
}

function stagger(progress, index, count) {
    const step = Math.min(MAX_STEP, SPREAD / Math.max(1, count - 1));
    const start = index * step;
    const span = 1 - step * Math.max(0, count - 1);
    return clamp01((progress - start) / span);
}

function easeOutCubic(value) {
    const inverse = 1 - clamp01(value);
    return 1 - inverse * inverse * inverse;
}

function hoverDuration(units) {
    return units.longDuration;
}

function moved(from, to) {
    return from.x !== to.x || from.y !== to.y;
}

function openDuration(units) {
    return units.longDuration * OPEN_FACTOR;
}

function shimmerDuration(units) {
    return units.longDuration * SHIMMER_FACTOR;
}

function spinDuration(units) {
    return units.longDuration * SPIN_FACTOR;
}

function spinRampAngle() {
    return TURN / 2;
}

function turnAngle() {
    return TURN;
}

function nextTurn(angle) {
    return Math.ceil(angle / TURN) * TURN;
}

function settleDuration(remaining, turnMs) {
    return Math.round(2 * Math.max(0, remaining) * turnMs / TURN);
}

function sheenSweepMs() {
    return SHEEN_SWEEP_MS;
}

function sheenRestMs() {
    return SHEEN_REST_MS;
}

function sheenActive(phase, fraction) {
    return phase > 0 && phase < 1 && fraction >= SHEEN_MIN_FILL;
}

function sheenOffset(phase, span, band) {
    return -band + clamp01(phase) * (span + band);
}

function sheenLighten() {
    return SHEEN_LIGHTEN;
}

function sheenPeak() {
    return SHEEN_PEAK;
}
