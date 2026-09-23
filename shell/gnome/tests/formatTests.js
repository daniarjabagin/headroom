import * as format from '../src/format.js';
import * as numbers from '../src/numbers.js';
import { check } from './check.js';

export function testFormat() {
    const now = new Date('2026-09-23T10:00:00Z');
    check('percentLeft', format.percentLeft(61.6), '62% left');
    check('percentLeft null', format.percentLeft(null), '—');
    check('reset hours', format.resetText(new Date('2026-09-23T12:41:00Z'), now), 'Resets in 2h 41m');
    check('reset days', format.resetText(new Date('2026-09-27T16:30:00Z'), now), 'Resets in 4d 6h');
    check('reset soon', format.resetText(new Date('2026-09-23T10:00:30Z'), now), 'Resets soon');
    check('not started', format.resetText(null, now), 'Not started');
    check('spare', format.spareText(4.2), '~4% spare');
    check('percent used', format.percentUsed(38.4), '38% used');
    check('reading left', format.readingPercent({ usedPercent: 38, remainingPercent: 62 }, 'left'), 62);
    check('reading used', format.readingPercent({ usedPercent: 38, remainingPercent: 62 }, 'used'), 38);
    check('reading percent used', format.readingPercent({ usedPercent: null, remainingPercent: 62 }, 'used'), 38);
    check('percent reading', format.percentReading(41.4, 'left'), '41% left');
    check('short session', format.shortWindowLabel('session', 'Session'), 'S');
    check('short weekly', format.shortWindowLabel('weekly', 'Weekly'), 'W');
    check('short model', format.shortWindowLabel('model:opus', 'Opus'), 'Opus');
    check('window label known', format.windowLabel('weekly', 'Weekly'), 'Weekly');
    check('window label model', format.windowLabel('model:opus', 'Opus'), 'Opus');
    check('limit', format.limitText(new Date('2026-09-24T19:00:00Z'), now), 'Limit in 1d 9h');
    check('next update', format.nextUpdateText(new Date('2026-09-23T10:03:10Z'), now), 'Next update in 3m');
    check('next update soon', format.nextUpdateText(new Date('2026-09-23T10:00:40Z'), now), 'Next update in <1m');
    testLiveCountdown(now);
}

function testLiveCountdown(now) {
    const soon = new Date('2026-09-23T10:12:07Z');
    check('live countdown', format.resetText(soon, now, 'countdown', true), 'Resets in 12m 07s');
    check('live seconds', format.resetText(new Date('2026-09-23T10:00:45Z'), now, 'countdown', true), 'Resets in 45s');
    check(
        'live over hour',
        format.resetText(new Date('2026-09-23T11:30:00Z'), now, 'countdown', true),
        'Resets in 1h 30m'
    );
    check('live is live', format.isCountdownLive(soon, now), true);
    check('live not past', format.isCountdownLive(new Date('2026-09-23T09:59:00Z'), now), false);
    check('live not far', format.isCountdownLive(new Date('2026-09-23T11:00:00Z'), now), false);
    check('live no reset', format.isCountdownLive(null, now), false);
}

export function testNumbers() {
    check('tokens K', numbers.compactTokens(1200), '1.2K');
    check('tokens M', numbers.compactTokens(35_812_904), '35.8M');
    check('tokens B', numbers.compactTokens(1_500_000_000), '1.5B');
    check('tokens small', numbers.compactTokens(999), '999');
    check('tokens round', numbers.compactTokens(3_000_000), '3M');
    check('usd', numbers.usd(14_370_000), '$14.37');
    check('usd K', numbers.usd(2_064_000_000), '$2.06K');
    check('usd cents', numbers.usd(4_050_000), '$4.05');
    check('exact usd', numbers.exactUsd(1_234_567_890), '$1,234.57');
    check('ring usd', numbers.ringUsd(463_120_000), '$463');
    check('ring usd small', numbers.ringUsd(18_420_000), '$18.42');
    check('spend line', numbers.spendLine({ costMicros: 4_080_000, totalTokens: 1_203_448 }), '$4.08 · 1.2M tokens');
    check('spend one token', numbers.spendLine({ costMicros: 10, totalTokens: 1 }), '$0.00 · 1 token');
    check('spend empty', numbers.spendLine({ costMicros: 0, totalTokens: 0 }), 'No data');
    check('exact tokens text', numbers.exactTokensText(1_203_448), '1,203,448 tokens');
    check(
        'exact spend line',
        numbers.exactSpendLine({ costMicros: 4_080_000, totalTokens: 12, partial: true }),
        '$4.08 · 12 tokens · some models unpriced'
    );
}
