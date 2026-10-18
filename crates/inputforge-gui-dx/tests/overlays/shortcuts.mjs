import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const {script, source, selectors, blocked} = JSON.parse(readFileSync(0, 'utf8'));
const handlers = new Set();
const window = {
  addEventListener(type, handler) { if (type === 'keydown') handlers.add(handler); },
  removeEventListener(type, handler) { if (type === 'keydown') handlers.delete(handler); },
};
class Element {
  tagName = 'BUTTON'; dataset = {navigationContext: 'profile:mapping', navigationActions: 'actions'};
  closest(selector) { return script === 'editor_shortcuts.js' && selector === '.if-editor' ? this : null; }
}
const element = new Element();
const document = {
  activeElement: element,
  querySelector(selector) {
    assert.ok(Object.hasOwn(selectors, selector), `selector was not evaluated against actual rendered HTML: ${selector}`);
    return selectors[selector] ? element : null;
  },
};
const sent = []; let receive;
const dioxus = {send(value) { sent.push(value); }, recv() { return new Promise(r => { receive = r; }); }};
// Desktop eval executes an AsyncFunction; its returned promise finishes independently
// of the long-lived event handler and pending shutdown receive.
await vm.runInNewContext(`(async()=>{${source.replaceAll('__LISTENER_KEY__', 'test-shortcut').replaceAll('__GENERATION__', '1')}})()`, {window, document, Element, dioxus});
for (const [key, shift] of script === 'editor_shortcuts.js' ? [['z', false], ['Z', true], ['y', false]] : [[script === 'snapshot_shortcut.js' ? 's' : 'f', false]]) {
  const event = {key, ctrlKey: true, shiftKey: shift, target: element, defaultPrevented: false, preventDefault() { this.defaultPrevented = true; }};
  const before = sent.length;
  for (const handler of handlers) handler(event);
  assert.equal(sent.length, before + (blocked ? 0 : 1), `${script}: ${blocked ? 'open' : 'closed'} rendered menu, Ctrl+${key}`);
}
window['test-shortcut'](); receive('__shutdown__');
await new Promise(r => setImmediate(r));
assert.equal(handlers.size, 0);
