import { needsOnboarding } from './compat.js';

const offered = new WeakSet();

export function maybeShowOnboarding(extension, state, settings) {
    if (offered.has(extension) || !needsOnboarding(state, settings)) return false;
    offered.add(extension);
    extension.openPreferences();
    return true;
}
