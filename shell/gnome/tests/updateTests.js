import { setLanguage } from '../src/i18n.js';
import { parseState } from '../src/state.js';
import {
    afterEvent,
    afterExit,
    IDLE,
    parseUpdate,
    runLine,
    showsWhatsNew,
    startedRun,
    updateAction,
    updateTitle,
} from '../src/update.js';
import { check } from './check.js';

const URL = 'https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0';

function raw(install, extra = {}) {
    return {
        version: '0.5.0',
        url: URL,
        published_at: '2026-09-20T12:00:00Z',
        install,
        command: 'yay -Syu headroom',
        ...extra,
    };
}

function testParse() {
    check('no update', parseUpdate(null), null);
    check('no version', parseUpdate({ url: URL }), null);
    check('full update', parseUpdate(raw('self')), {
        version: '0.5.0',
        url: URL,
        publishedAt: new Date('2026-09-20T12:00:00Z'),
        install: 'self',
        command: 'yay -Syu headroom',
    });
    check('unknown install kind', parseUpdate(raw('snap')).install, 'unknown');
    check('non-https url dropped', parseUpdate(raw('self', { url: 'javascript:alert(1)' })).url, null);
    check('blank command', parseUpdate(raw('package', { command: '  ' })).command, null);
    check('state without update', parseState('{"version": 1}').update, null);
    check('state update', parseState(JSON.stringify({ version: 1, update: raw('package') })).update.install, 'package');
}

function testActions() {
    const kinds = ['self', 'package', 'unknown'].map(kind => updateAction(parseUpdate(raw(kind))).kind);
    check('action per install kind', kinds, ['install', 'command', 'notes']);
    check('package without command', updateAction(parseUpdate(raw('package', { command: null }))).kind, 'notes');
    check('nothing to offer', updateAction(parseUpdate(raw('unknown', { url: null }))), null);
    const whatsNew = ['self', 'package', 'unknown'].map(kind => showsWhatsNew(parseUpdate(raw(kind))));
    check('whats new beside own action', whatsNew, [true, true, false]);
    check('no whats new without url', showsWhatsNew(parseUpdate(raw('self', { url: null }))), false);
}

function testRun() {
    const running = startedRun();
    check('idle has no line', runLine(IDLE), null);
    check('starting line', runLine(running), 'Starting the update…');
    const step = afterEvent(running, { event: 'step', text: 'Downloading headroom 0.5.0' });
    check('step line', runLine(step), 'Downloading headroom 0.5.0');
    check('output ignored', afterEvent(step, { event: 'output', line: 'noise' }), step);
    const done = afterEvent(step, { event: 'done', version: '0.5.0', relogin: true });
    check('done relogin', runLine(done), 'Updated — log out and back in to finish');
    const quiet = afterEvent(step, { event: 'done', version: '0.5.0', relogin: false });
    check('done without relogin', runLine(quiet), 'Updated to 0.5.0');
    check('exit after done keeps done', afterExit(done, null), done);
    const failed = afterEvent(step, { event: 'error', message: 'checksum mismatch' });
    check('error line', [failed.phase, runLine(failed)], ['failed', 'checksum mismatch']);
    check('exit after error keeps message', afterExit(failed, 'headroom exited with status 1'), failed);
    check('exit without done', runLine(afterExit(step, null)), 'The update stopped before it finished');
    check('exit status', runLine(afterExit(step, 'headroom exited with status 2')), 'headroom exited with status 2');
}

function testTexts() {
    const update = parseUpdate(raw('self'));
    check('title', updateTitle(update), 'Headroom 0.5.0 is available');
    setLanguage('ru');
    check('title ru', updateTitle(update), 'Доступна версия Headroom 0.5.0');
    const relogin = runLine({ phase: 'done', version: '0.5.0', relogin: true });
    check('relogin ru', relogin, 'Обновлено — выйдите из сеанса и войдите снова');
    setLanguage('en');
}

export function testUpdate() {
    testParse();
    testActions();
    testRun();
    testTexts();
}
