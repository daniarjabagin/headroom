import GLib from 'gi://GLib';
import { ProgressProcess } from '../src/prefs/cli.js';
import { check } from './check.js';

const FAKE_HEADROOM = `#!/bin/sh
marker="$HEADROOM_TEST_DIR/$1"
echo $$ > "$marker.pid"
case "$1" in
    graceful) trap 'echo term > "$marker.signal"; exit 0' TERM ;;
    stubborn) trap 'echo term > "$marker.signal"' TERM ;;
    failing) echo '{"event":"output","line":"bad"}'; exit 3 ;;
    apikey)
        printf '%s\n' "$@" > "$marker.args"
        IFS= read -r key
        printf '%s' "$key" > "$marker.key"
        [ -z "$(cat)" ] || exit 4
        echo '{"event":"done","account_id":"grok:1"}'
        exit 0 ;;
esac
echo '{"event":"started","provider":"codex"}'
while :; do sleep 0.05; done
`;

function delay(ms) {
    return new Promise(resolve =>
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
            resolve();
            return GLib.SOURCE_REMOVE;
        })
    );
}

async function until(predicate, timeoutMs) {
    for (let waited = 0; waited < timeoutMs; waited += 25) {
        if (predicate()) return true;
        await delay(25);
    }
    return predicate();
}

function installFakeHeadroom() {
    const dir = GLib.dir_make_tmp('headroom-cli-XXXXXX');
    const script = GLib.build_filenamev([dir, 'headroom']);
    GLib.file_set_contents(script, FAKE_HEADROOM);
    GLib.chmod(script, 0o755);
    GLib.setenv('HEADROOM_TEST_DIR', dir, true);
    GLib.setenv('PATH', `${dir}:${GLib.getenv('PATH')}`, true);
    return dir;
}

function readMarker(dir, name) {
    const path = GLib.build_filenamev([dir, name]);
    if (!GLib.file_test(path, GLib.FileTest.EXISTS)) return null;
    const [, bytes] = GLib.file_get_contents(path);
    return new TextDecoder().decode(bytes).trim();
}

function isAlive(pid) {
    return GLib.file_test(`/proc/${pid}`, GLib.FileTest.EXISTS);
}

function start(mode, events, exits) {
    return new ProgressProcess([mode], {
        onEvent: event => events.push(event.event),
        onExit: error => exits.push(error),
    });
}

async function testGracefulCancel(dir) {
    const events = [];
    const exits = [];
    const process = start('graceful', events, exits);
    await until(() => events.includes('started'), 2000);
    const pid = readMarker(dir, 'graceful.pid');
    process.cancel();
    check('cancel sends SIGTERM', await until(() => readMarker(dir, 'graceful.signal') === 'term', 2000), true);
    check('cancel lets child exit', await until(() => !isAlive(pid), 2000), true);
    check('cancel reports no exit', exits, []);
}

async function testStubbornCancel(dir) {
    const events = [];
    const process = start('stubborn', events, []);
    await until(() => events.includes('started'), 2000);
    const pid = readMarker(dir, 'stubborn.pid');
    process.cancel();
    await until(() => readMarker(dir, 'stubborn.signal') === 'term', 2000);
    check('stubborn survives SIGTERM', isAlive(pid), true);
    check('stubborn killed after grace', await until(() => !isAlive(pid), 4000), true);
}

async function testFailingExit() {
    const events = [];
    const exits = [];
    start('failing', events, exits);
    await until(() => exits.length > 0, 3000);
    check('failing output', events, ['output']);
    check('failing status', exits, ['headroom exited with status 3']);
}

async function testKeyOnStdin(dir) {
    const events = [];
    const exits = [];
    const process = start('apikey', events, exits);
    process.write('sk-test-123');
    process.closeInput();
    process.write('ignored after close');
    await until(() => exits.length > 0, 3000);
    check('key arrives on stdin', readMarker(dir, 'apikey.key'), 'sk-test-123');
    check('key never in argv', readMarker(dir, 'apikey.args').includes('sk-test'), false);
    check('stdin closed after key', [events, exits], [['done'], [null]]);
}

function removeDir(dir) {
    const names = [
        'headroom',
        'apikey.key',
        'apikey.args',
        ...['graceful', 'stubborn', 'failing', 'apikey'].flatMap(mode => [`${mode}.pid`, `${mode}.signal`]),
    ];
    for (const name of names) GLib.unlink(GLib.build_filenamev([dir, name]));
    GLib.rmdir(dir);
}

export async function testProgressProcess() {
    const dir = installFakeHeadroom();
    try {
        await testGracefulCancel(dir);
        await testStubbornCancel(dir);
        await testFailingExit();
        await testKeyOnStdin(dir);
    } finally {
        removeDir(dir);
    }
}
