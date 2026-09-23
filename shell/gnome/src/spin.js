export const FULL_TURN = 360;
export const QUARTER_TURN = 90;
export const TURN_MS = 1400;
export const EASE_MS = Math.round(((2 * QUARTER_TURN) / FULL_TURN) * TURN_MS);

export function restingAngle(angle) {
    return ((angle % FULL_TURN) + FULL_TURN) % FULL_TURN;
}

export function stopPlan(angle) {
    const from = restingAngle(angle);
    if (from === 0) return null;
    const left = FULL_TURN - from;
    const distance = left < QUARTER_TURN ? left + FULL_TURN : left;
    const cruise = distance - QUARTER_TURN;
    return {
        from,
        cruiseTo: from + cruise,
        cruiseMs: Math.round((cruise / FULL_TURN) * TURN_MS),
        target: from + distance,
    };
}
