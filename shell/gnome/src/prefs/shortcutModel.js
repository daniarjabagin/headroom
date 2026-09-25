import { isAccelerator } from '../settings.js';

const MODIFIER_KEY =
    /^(?:(?:Shift|Control|Alt|Meta|Super|Hyper)_[LR]|ISO_Level\d_(?:Shift|Latch|Lock)|Caps_Lock|Num_Lock)$/;

export function captureOutcome({ keyName, hasModifiers, accelerator }) {
    if (!hasModifiers && keyName === 'Escape') return { kind: 'cancel' };
    if (!hasModifiers && keyName === 'BackSpace') return { kind: 'disable' };
    if (!hasModifiers || MODIFIER_KEY.test(keyName ?? '')) return { kind: 'wait' };
    if (!accelerator || !isAccelerator(accelerator)) return { kind: 'wait' };
    return { kind: 'set', accelerator };
}
