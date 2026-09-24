import { _ } from './i18n.js';

const LABEL_TEXTS = () => ({
    Credits: _('Credits'),
    'Extra usage': _('Extra usage'),
    Balance: _('Balance'),
    Vouchers: _('Vouchers'),
    Cash: _('Cash'),
    'Credit balance': _('Credit balance'),
    'Organization credits': _('Organization credits'),
    'Point balance': _('Point balance'),
    'Bonus credits': _('Bonus credits'),
    'Monthly credits': _('Monthly credits'),
});

export function labelText(label) {
    const texts = LABEL_TEXTS();
    return typeof label === 'string' && Object.hasOwn(texts, label) ? texts[label] : label;
}
