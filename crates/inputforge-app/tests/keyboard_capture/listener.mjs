// Exercise the production script's interception and synchronous cleanup.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const source = fs.readFileSync(process.argv[2], 'utf8');
const handlers = new Map();
const window = {
  addEventListener(kind, fn, capture) {
    assert.equal(capture, true);
    const list = handlers.get(kind) ?? new Set(); list.add(fn); handlers.set(kind, list);
  },
  removeEventListener(kind, fn, capture) { assert.equal(capture, true); handlers.get(kind)?.delete(fn); },
};
function install(key) {
  const sent = []; let shutdown;
  vm.runInNewContext(`{${source.replaceAll('__LISTENER_KEY__', key)}}`, {
    window, console, dioxus: { send: value => sent.push(value), recv: () => new Promise(resolve => { shutdown = resolve; }) },
  });
  return { sent, shutdown: () => shutdown('__shutdown__') };
}
function fire(kind, key, code) {
  const ev = { key, code, location: 2, ctrlKey: true, altKey: false, shiftKey: false, metaKey: false,
    prevented: 0, stopped: 0, immediate: 0, preventDefault() { this.prevented++; }, stopPropagation() { this.stopped++; }, stopImmediatePropagation() { this.immediate++; } };
  for (const h of [...(handlers.get(kind) ?? [])]) { h(ev); if (ev.immediate) break; }
  return ev;
}
const first = install('if-keyboard-capture-test');
assert.equal(typeof window['if-keyboard-capture-test'], 'function', 'cleanup must be callable synchronously');
assert.equal(fire('keydown', 's', 'KeyS').prevented, 0, 'installed listener must be passive before arming');
let shortcuts = 0;
const shortcut = () => { shortcuts++; };
window.addEventListener('keydown', shortcut, true);
window['if-keyboard-capture-test'].arm(7);
const event = fire('keydown', 's', 'KeyS');
assert.equal(event.prevented, 1); assert.equal(event.stopped, 1); assert.equal(event.immediate, 1);
assert.equal(shortcuts, 0, 'capture must suppress later same-window shortcut handlers');
window.removeEventListener('keydown', shortcut, true);
assert.deepEqual(JSON.parse(JSON.stringify(first.sent[0])), {
  serial: 7, kind: 'keydown', key: 's', code: 'KeyS', location: 2, ctrl: true, alt: false, shift: false, meta: false,
});
fire('keyup', 's', 'KeyS');
assert.equal(first.sent.length, 2);
window['if-keyboard-capture-test'].disarm();
assert.equal(fire('keydown', 's', 'KeyS').prevented, 0);
const second = install('if-keyboard-capture-other');
window['if-keyboard-capture-other'].arm(8);
window['if-keyboard-capture-test']();
assert.equal(handlers.get('keydown').size, 1, 'cleanup must leave other listeners intact');
fire('keydown', 'Escape', 'Escape');
assert.equal(first.sent.length, 2); assert.equal(second.sent.length, 1);
window['if-keyboard-capture-other']();
assert.equal(fire('keydown', 's', 'KeyS').prevented, 0, 'idle keyboard input must remain passive');
first.shutdown(); second.shutdown();
await new Promise(resolve => setImmediate(resolve));
assert.equal(first.sent.at(-1), 'shutdown_ack');
assert.equal(second.sent.at(-1), 'shutdown_ack');
assert.equal(handlers.get('keydown').size, 0); assert.equal(handlers.get('keyup').size, 0);
console.log('keyboard listener interception, isolated cleanup, idle behavior and shutdown passed');
