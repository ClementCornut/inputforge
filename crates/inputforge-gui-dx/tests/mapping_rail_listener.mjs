// Execute the production listener bodies with controlled browser events.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const rail = readFileSync('crates/inputforge-gui-dx/src/frame/mapping_list/mod.rs', 'utf8');
const add = readFileSync('crates/inputforge-gui-dx/src/frame/mapping_list/add_inline.rs', 'utf8');
const keyboard = rail.match(/r#"([\s\S]*?)"#/)[1];
const escape = add.match(/r"([\s\S]*?)"/)[1].replaceAll('{{', '{').replaceAll('}}', '}');
const handlers = new Set();
const window = {
  addEventListener(kind, fn, capture) { assert.equal(kind, 'keydown'); assert.equal(capture, true); handlers.add(fn); },
  removeEventListener(kind, fn, capture) { assert.equal(kind, 'keydown'); assert.equal(capture, true); handlers.delete(fn); },
};
class Element {
  constructor(editable = false, filter = false) { this.editable = editable; this.filter = filter; }
  closest(selector) { return selector === '.if-rail__filter' ? this.filter : this.editable; }
}
const document = { activeElement: null, menu: false, querySelector() { return this.menu ? {} : null; } };
function install(body, key) {
  const messages = [];
  let receive;
  const dioxus = { send(data) { messages.push(structuredClone(data)); }, recv() { return new Promise(resolve => { receive = resolve; }); } };
  vm.runInNewContext(body.replaceAll('__key__', key).replaceAll('{key}', key), { window, document, Element, dioxus });
  return { messages, stop() { receive('__shutdown__'); } };
}
function key(value, target = new Element(), alt = false) {
  for (const handle of handlers) handle({ key: value, target, altKey: alt, metaKey: false, ctrlKey: false });
}
const first = install(keyboard, 'old-rail');
const pad = install(escape, 'pad');
key('ArrowUp', new Element(), true);
assert.deepEqual(first.messages, [['ArrowUp', 0, 0, 1]]);
window.__ifSortableDrag = {};
key('ArrowDown', new Element(), true);
assert.equal(first.messages.length, 1, 'A pointer drag must not enqueue keyboard reorders');
delete window.__ifSortableDrag;
key('ArrowDown', new Element(true));
document.activeElement = new Element(true);
key('ArrowDown');
document.activeElement = null;
document.menu = true;
key('ArrowDown');
document.menu = false;
assert.equal(first.messages.length, 1);
key('Escape', new Element(true, true));
assert.equal(pad.messages.length, 0);
key('Escape');
assert.deepEqual(pad.messages, ['esc']);
// Teardown dispatches removal before waking the receiver; later mounts are isolated.
window['old-rail'](); delete window['old-rail'];
window.pad(); delete window.pad;
assert.equal(handlers.size, 0);
const fresh = install(keyboard, 'new-rail');
first.stop(); pad.stop();
await new Promise(resolve => setImmediate(resolve));
assert.equal(handlers.size, 1);
key('ArrowDown');
assert.deepEqual(fresh.messages, [['ArrowDown', 0, 0, 0]]);
window['new-rail'](); delete window['new-rail']; fresh.stop();
await new Promise(resolve => setImmediate(resolve));
assert.equal(handlers.size, 0);
const editorSource = readFileSync('crates/inputforge-gui-dx/src/frame/mapping_editor/keyboard.rs', 'utf8');
const editorStart = editorSource.indexOf('const h = (ev) => {');
const editor = install(editorSource.slice(editorStart, editorSource.indexOf('\n";', editorStart)), 'editor');
key('ArrowUp');
assert.equal(editor.messages.length, 1);
window.__ifSortableDrag = {};
key('ArrowDown');
assert.equal(editor.messages.length, 1, 'Editor keyboard commands must defer during a pointer drag');
delete window.__ifSortableDrag;
editor.stop();
await new Promise(resolve => setImmediate(resolve));
assert.equal(handlers.size, 0);
process.stdout.write('Rail/editor listener checks passed.\n');
