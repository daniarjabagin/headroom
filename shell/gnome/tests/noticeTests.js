import { setLanguage } from '../src/i18n.js';
import { noticeKind, noticeText } from '../src/notices.js';
import { check } from './check.js';

function testKinds() {
    check('notice kinds by tone', ['good', 'neutral', 'warning', 'critical'].map(noticeKind), [
        'info',
        'info',
        'warning',
        'error',
    ]);
}

function testEnglishTexts() {
    check('en fixed text kept', noticeText('No Cline credits left.'), 'No Cline credits left.');
    check('en pattern kept', noticeText('Extra usage on, cap 12.5'), 'Extra usage on, cap 12.5');
    check('en unknown kept', noticeText('Something new'), 'Something new');
}

function testRussianTexts() {
    check('ru fixed text', noticeText('Weekly limit shared with Codex Cloud'), 'Недельный лимит общий с Codex Cloud');
    check('ru extra usage cap', noticeText('Extra usage on, cap 12.5'), 'Доп. использование включено, предел 12.5');
    check('ru plan renews', noticeText('Pro renews on 2026-10-01 (UTC).'), 'Pro продлевается 2026-10-01 (UTC).');
    check(
        'ru plan ends',
        noticeText('Cline Pro ends on 2026-10-01 (UTC).'),
        'Cline Pro заканчивается 2026-10-01 (UTC).'
    );
    check('ru unknown kept', noticeText('Something new'), 'Something new');
    check('ru prototype key kept', noticeText('constructor'), 'constructor');
}

export function testNotices() {
    testKinds();
    testEnglishTexts();
    setLanguage('ru');
    try {
        testRussianTexts();
    } finally {
        setLanguage('en');
    }
}
