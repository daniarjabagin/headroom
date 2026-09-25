const RELEASE = /^(\d+)\.(\d+)\.(\d+)/;
const FIRST_06 = [0, 6, 0];

function releaseParts(version) {
    const match = typeof version === 'string' ? RELEASE.exec(version) : null;
    return match ? match.slice(1, 4).map(Number) : null;
}

export function isReleaseAtLeast(version, minimum) {
    const parts = releaseParts(version);
    if (parts === null) return false;
    const index = parts.findIndex((part, position) => part !== minimum[position]);
    return index === -1 || parts[index] > minimum[index];
}

export function supports06(state) {
    if (!state) return false;
    return state.reportsPanelItems === true || isReleaseAtLeast(state.appVersion, FIRST_06);
}

export function needsOnboarding(state, settings) {
    return supports06(state) && settings?.onboarding.completed === false;
}
