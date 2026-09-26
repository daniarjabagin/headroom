export const SWEEP_MS = 1600;
export const REST_MS = 3500;
export const START_DELAY_MS = 600;
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
