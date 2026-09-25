import { isAccelerator } from './settingsValues.js';

export function grabbable(accelerator) {
    return typeof accelerator === 'string' && accelerator !== '' && isAccelerator(accelerator) ? accelerator : '';
}

export function shortcutChange(current, requested) {
    const next = grabbable(requested);
    if (next === current) return null;
    return { release: current !== '', grab: next };
}
