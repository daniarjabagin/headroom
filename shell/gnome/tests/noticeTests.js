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
    check('ru kilo used up', noticeText('Kilo credits are used up'), 'Кредиты Kilo закончились');
    check('ru warp unlimited', noticeText('Unlimited credits'), 'Безлимитные кредиты');
    check(
        'ru warp no credits',
        noticeText('No monthly credits on this plan'),
        'В этом тарифе нет ежемесячных кредитов'
    );
    check(
        'ru deepseek unavailable',
        noticeText('Balance is not enough for API calls'),
        'Баланса не хватает для вызовов API'
    );
    check(
        'ru deepseek used up',
        noticeText('Balance is used up; API calls fail until you top up'),
        'Баланс исчерпан — вызовы API не пройдут, пока вы не пополните счёт'
    );
    check(
        'ru moonshot used up',
        noticeText('Balance is used up; API requests fail until you top up'),
        'Баланс исчерпан — запросы к API не пройдут, пока вы не пополните счёт'
    );
    check(
        'ru moonshot debt',
        noticeText('Cash balance is negative: the account is in debt'),
        'Денежный баланс отрицательный — на счёте долг'
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
