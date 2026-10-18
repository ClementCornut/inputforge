import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const script = readFileSync(new URL('../../assets/components/navigation/measure_cursor_menu.js', import.meta.url), 'utf8');

test('cursor menu measures layout after the mounted node and portal commit', () => {
    const frames = [];
    const sent = [];
    let committed = false;
    const menu = {
        getBoundingClientRect() {
            return committed ? { width: 240, height: 320 } : { width: 0, height: 0 };
        },
    };
    const measure = vm.runInNewContext(`(${script})`, {
        document: { getElementById: () => menu },
        window: { innerWidth: 800, innerHeight: 600 },
        requestAnimationFrame: (callback) => frames.push(callback),
        dioxus: { send: (value) => sent.push(Array.from(value)) },
    });
    measure('mapping-actions');
    assert.deepEqual(sent, [], 'measurement must wait for the browser layout frame');
    assert.equal(frames.length, 1, 'one finite layout frame is sufficient after native mount');
    committed = true;
    frames.shift()();
    assert.deepEqual(sent, [[240, 320, 800, 600]]);
    assert.equal(frames.length, 0, 'measurement must not leave a polling loop');
});
