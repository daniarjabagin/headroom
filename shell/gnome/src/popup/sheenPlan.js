export const SWEEP_MS = 1400;
export const REST_MS = 5000;
export const START_DELAY_MS = 600;
export const STEP_MS = 33;
export const QUIET_MS = 1200;
const MIN_FRACTION = 0.03;

export function showsSheen(fraction, tone) {
    return tone !== 'none' && fraction >= MIN_FRACTION;
}

export function sheenClip(fillX, fillWidth, height) {
    const inset = height / 2;
    return { x: fillX + inset, width: Math.max(0, fillWidth - 2 * inset) };
}

export function sheenOffset(progress, clipWidth, bandWidth) {
    return -bandWidth + progress * (clipWidth + bandWidth);
}

export function sheenStrength(offset, clipWidth, bandWidth) {
    if (bandWidth <= 0) return 0;
    const overlap = Math.min(offset + bandWidth, clipWidth) - Math.max(offset, 0);
    return Math.min(1, Math.max(0, overlap / bandWidth));
}

export function sweepProgress(elapsedMs) {
    const linear = Math.min(1, Math.max(0, elapsedMs / SWEEP_MS));
    return (1 - Math.cos(Math.PI * linear)) / 2;
}

export function pointerQuiet(sinceInteractionMs) {
    return sinceInteractionMs >= QUIET_MS;
}
