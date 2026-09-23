.pragma library

const MAX_STEP = 0.12;
const SPREAD = 0.45;
const OPEN_FACTOR = 3;
const PULSE_FACTOR = 6;
const SHIMMER_FACTOR = 7;

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

function openDuration(units) {
    return units.longDuration * OPEN_FACTOR;
}

function pulseDuration(units) {
    return units.longDuration * PULSE_FACTOR;
}

function shimmerDuration(units) {
    return units.longDuration * SHIMMER_FACTOR;
}
