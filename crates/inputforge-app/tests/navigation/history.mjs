import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

// Parent links model the actual editor/stage/plot tree, rather than treating
// each synthetic focus target as the editor element itself.
class Element {
  constructor(tag, parentElement = null, attrs = {}) {
    this.tagName = tag.toUpperCase(); this.parentElement = parentElement;
    this.attrs = attrs; this.type = attrs.type ?? 'text';
    this.dataset = attrs.dataset ?? {};
    this.isContentEditable = attrs.contenteditable === 'true' || Boolean(parentElement?.isContentEditable);
  }
  matches(selectors) {
    return selectors.split(',').some(raw => {
      const s = raw.trim();
      if (s === '.if-editor' || s === '.if-rail') return this.attrs.class === s.slice(1);
      if (s.startsWith('.if-stage')) return this.attrs.class === 'if-stage' && this.dataset.stageId !== undefined;
      if (s === '[contenteditable]') return this.attrs.contenteditable !== undefined;
      if (s === '[role="textbox"]') return this.attrs.role === 'textbox';
      const input = s.match(/^input\[type="(.*?)"\]$/);
      if (input) return this.tagName === 'INPUT' && this.type === input[1];
      return this.tagName.toLowerCase() === s;
    });
  }
  closest(selector) { return this.matches(selector) ? this : this.parentElement?.closest(selector) ?? null; }
  getBoundingClientRect() { return {left: 4, bottom: 9}; }
}
const handlers = new Set();
const window = {
  addEventListener(type, h, capture) { assert.equal(capture, true); if (type === 'keydown') handlers.add(h); },
  removeEventListener(type, h) { if (type === 'keydown') handlers.delete(h); },
};
const body = new Element('body');
const editor = new Element('div', body, {class: 'if-editor', dataset: {navigationContext: 'profile:mapping', navigationActions: 'rendered actions'}});
const stage = new Element('div', editor, {class: 'if-stage', dataset: {stageId: '0'}});
const document = {
  body, activeElement: body, mounted: true, modal: false,
  querySelector(s) { return s === '.if-editor' ? (this.mounted ? editor : null) : this.modal ? {} : null; },
};
function install(key) {
  const messages = []; let receive;
  const dioxus = {send(message) { messages.push(structuredClone(message)); }, recv() { return new Promise(r => { receive = r; }); }};
  vm.runInNewContext(readFileSync('crates/inputforge-app/assets/editor_shortcuts.js', 'utf8').replaceAll('__LISTENER_KEY__', key), {window, document, Element, dioxus});
  return {messages, stop() { window[key]?.(); delete window[key]; receive('__shutdown__'); }};
}
function fire(key = 'z', opts = {}) {
  const e = {key, target: document.activeElement, ctrlKey: true, shiftKey: false, altKey: false, metaKey: false, defaultPrevented: false, preventDefault() { this.defaultPrevented = true; }, ...opts};
  for (const handler of [...handlers]) handler(e);
  return e.defaultPrevented;
}
const listener = install('history');
function history(target) {
  document.activeElement = target;
  for (const [key, shift] of [['z', false], ['Z', true], ['y', false]]) {
    const count = listener.messages.length;
    assert.equal(fire(key, {shiftKey: shift}), true, `${target.tagName}/${target.type}: history must reach app`);
    assert.equal(listener.messages.length, count + 1);
    assert.deepEqual(listener.messages.at(-1).slice(8), ['profile:mapping', 'rendered actions']);
  }
}
history(body);
assert.equal(fire('z', {ctrlKey: false, metaKey: true}), true);
assert.equal(fire('z', {altKey: true}), false);
history(new Element('button', new Element('aside', body, {class: 'if-rail'})));
document.activeElement = new Element('button', new Element('aside', body));
assert.equal(fire(), false, 'unrelated side-panel controls must not undo mapping history');
history(new Element('path', new Element('svg', stage)));
history(new Element('div', stage)); // Focused plot wrapper.
history(new Element('input', stage, {type: 'checkbox'}));
history(new Element('input', stage, {type: 'radio'}));
history(new Element('input', stage, {type: 'range'}));
history(new Element('select', stage));
history(new Element('option', new Element('select', stage)));
for (const parent of [stage, body]) {
  for (const field of [new Element('input', parent), new Element('input', parent, {type: 'number'}), new Element('textarea', parent), new Element('div', parent, {role: 'textbox'}), new Element('span', new Element('div', parent, {contenteditable: 'true'}))]) {
    document.activeElement = field;
    for (const [key, shift] of [['z', false], ['Z', true], ['y', false]]) {
      const count = listener.messages.length;
      assert.equal(fire(key, {shiftKey: shift}), false, 'native text history must remain available');
      assert.equal(listener.messages.length, count);
    }
  }
}
document.activeElement = body;
assert.equal(fire('ArrowUp', {ctrlKey: false, altKey: true}), false, 'global history must not globalize stage shortcuts');
assert.equal(fire('F10', {ctrlKey: false, shiftKey: true}), false);
document.modal = true; assert.equal(fire(), false); document.modal = false;
window.__ifSortableDrag = {}; assert.equal(fire(), false); delete window.__ifSortableDrag;
const beforePrevented = listener.messages.length;
assert.equal(fire('z', {defaultPrevented: true}), true);
assert.equal(listener.messages.length, beforePrevented);
document.mounted = false; assert.equal(fire(), false, 'no editor on another main surface'); document.mounted = true;
listener.stop(); assert.equal(handlers.size, 0);
const next = install('history-remount'); document.activeElement = stage;
assert.equal(fire(), true); assert.equal(next.messages.length, 1);
next.stop(); await new Promise(r => setImmediate(r)); assert.equal(handlers.size, 0);
process.stdout.write('Real DOM-shape history transport, native text history, surface gates and remount checks passed.\n');
