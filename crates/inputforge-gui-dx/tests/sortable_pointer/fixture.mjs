import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import vm from 'node:vm';
const path = 'crates/inputforge-app/assets/sortable_drag.js';
const script = existsSync(path) ? readFileSync(path, 'utf8') : '';
class ClassList extends Set { contains(x) { return this.has(x); } remove(x) { this.delete(x); } }
class Surface {
  listeners = new Map();
  addEventListener(kind, fn) { if (!this.listeners.has(kind)) this.listeners.set(kind, new Set()); this.listeners.get(kind).add(fn); }
  removeEventListener(kind, fn) { this.listeners.get(kind)?.delete(fn); }
  emit(kind, event = {}) { event.type = kind; for (const fn of this.listeners.get(kind) ?? []) fn(event); }
  listenerCount() { return [...this.listeners.values()].reduce((n, s) => n + s.size, 0); }
}
class Node extends Surface {
  dataset = {}; children = []; classList = new ClassList(); isConnected = true; captured = null;
  style = {}; extraAttributes = new Map(); textContent = '';
  constructor(id, top, height, parent = null) {
    super(); Object.assign(this, { id, top, height, parentElement: parent, clientTop: 0, clientHeight: height });
    parent?.children.push(this);
  }
  matches(selector) {
    if (selector === '[data-sortable-group]') return this.dataset.sortableGroup !== undefined;
    if (selector === '[data-sortable-row]') return this.dataset.sortableRow !== undefined;
    return selector === '.if-sortable-gap' && this.classList.contains('if-sortable-gap');
  }
  closest(selector) { return this.matches(selector) ? this : this.parentElement?.closest(selector) ?? null; }
  get attributes() {
    return [{ name: 'id', value: this.id }, ...this.extraAttributes.entries().map(([name, value]) => ({ name, value })),
      ...Object.entries(this.dataset).map(([key, value]) => ({ name: 'data-' + key.replace(/[A-Z]/g, c => '-' + c.toLowerCase()), value }))];
  }
  setAttribute(name, value) { this.extraAttributes.set(name, value); }
  removeAttribute(name) {
    if (name === 'id') this.id = '';
    this.extraAttributes.delete(name);
    if (name.startsWith('data-')) delete this.dataset[name.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase())];
  }
  querySelectorAll(selector) {
    const descendants = this.children.flatMap(n => [n, ...n.querySelectorAll('*')]);
    return selector === '*' ? descendants : descendants.filter(n => n.tagName?.toLowerCase() === selector);
  }
  cloneNode(deep) {
    const copy = new this.constructor(this.id, this.top, this.height);
    copy.dataset = { ...this.dataset }; copy.style = { ...this.style }; copy.extraAttributes = new Map(this.extraAttributes);
    copy.classList = new ClassList(this.classList); copy.textContent = this.textContent;
    if (deep) for (const child of this.children) copy.appendChild(child.cloneNode(true));
    return copy;
  }
  appendChild(child) { child.parentElement = this; child.isConnected = true; this.children.push(child); return child; }
  remove() { this.parentElement.children.splice(this.parentElement.children.indexOf(this), 1); this.parentElement = null; this.isConnected = false; }
  get previousElementSibling() { return this.parentElement?.children[this.parentElement.children.indexOf(this) - 1] ?? null; }
  get nextElementSibling() { return this.parentElement?.children[this.parentElement.children.indexOf(this) + 1] ?? null; }
  getBoundingClientRect() {
    let offset = 0; for (let p = this.parentElement; p; p = p.parentElement) offset += p.scrollTop ?? 0;
    return { top: this.top - offset, bottom: this.top + this.height - offset, height: this.height, left: 0, right: 300, width: 300 };
  }
  setPointerCapture(id) { this.captured = id; }
  hasPointerCapture(id) { return this.captured === id; }
  releasePointerCapture() { this.captured = null; }
}
class Canvas extends Node {
  tagName = 'CANVAS'; draws = [];
  getContext(kind) { assert.equal(kind, '2d'); return { drawImage: (...args) => this.draws.push(args) }; }
}
class DragEvent { constructor(type, init) { Object.assign(this, init, { type }); } }
class DataTransfer { setData() {} }
function event(x, y, pointerId = 1) {
  return { clientX: x, clientY: y, pointerId, button: 0, buttons: 1, isPrimary: true, defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; }, stopPropagation() {}, stopImmediatePropagation() {} };
}
export function fixture(height = 300) {
  const view = new Surface(), frames = new Map(), events = [];
  let serial = 0;
  view.requestAnimationFrame = fn => { frames.set(++serial, fn); return serial; };
  view.cancelAnimationFrame = id => frames.delete(id);
  const body = new Node('body', 0, height), pane = new Node('pane', 0, height, body);
  pane.scrollTop = 0; pane.scrollHeight = 500; pane.style = { overflowY: 'auto' };
  const group = new Node('group', 20, 212, pane); group.dataset.sortableGroup = 'owner';
  group.dataset.sortableContext = 'before';
  const gaps = [], rows = [];
  for (const [i, top, size] of [[0, 20, 40], [1, 68, 80], [2, 156, 60]]) {
    const gap = new Node(`gap-${i}`, top, 8, group); gap.classList.add('if-sortable-gap'); gaps.push(gap);
    const row = new Node(`row-${i}`, top + 8, size, group); row.dataset.sortableRow = ''; rows.push(row);
  }
  const last = new Node('gap-3', 224, 8, group); last.classList.add('if-sortable-gap'); gaps.push(last);
  const source = new Node('handle', 28, 16, rows[0]);
  source.dataset.sortableOwner = 'owner'; source.dataset.sortableEnabled = 'true';
  source.dataset.sortableContext = 'before';
  rows[0].textContent = 'Mapping card summary';
  rows[0].dataset.stageId = '0';
  source.setAttribute('data-dioxus-id', 'native-handle');
  source.setAttribute('onpointerdown', 'native handler');
  const field = new Node('field-id', 28, 12, rows[0]);
  field.setAttribute('name', 'mode'); field.setAttribute('autofocus', '');
  const document = { body, defaultView: view, elementFromPoint(x, y) {
    if (x < 0 || x > 300 || y < 0 || y >= height) return body;
    function hit(n) {
      const r = n.getBoundingClientRect();
      if (!n.isConnected || y < r.top || y >= r.bottom) return null;
      return n.children.map(hit).find(Boolean) ?? n;
    }
    return hit(group) ?? pane;
  } };
  for (const n of [source, pane, group, ...gaps, ...rows]) {
    n.ownerDocument = document;
    n.dispatchEvent = e => { events.push([e.type, n.id]); return true; };
  }
  const context = vm.createContext({ window: view, document, DragEvent, DataTransfer,
    getComputedStyle: n => n.style ?? { overflowY: 'visible' } });
  vm.runInContext(script, context);
  function frame(time) { const pending = [...frames.values()]; frames.clear(); for (const fn of pending) fn(time); }
  return { source, pane, group, gaps, events, frame,
    ghost() { return body.children.find(n => n.classList.contains('if-sortable-ghost')); },
    canvas() { return new Canvas('canvas-id', 28, 20, rows[0]); },
    emptyBranch() {
      const branch = new Node('branch', 104, 40, rows[1]); branch.dataset.sortableGroup = 'owner';
      branch.dataset.sortableContext = 'before';
      const gap = new Node('branch-gap', 104, 8, branch); gap.classList.add('if-sortable-gap');
      new Node('branch-add', 112, 32, branch);
      gap.dispatchEvent = e => { events.push([e.type, gap.id]); return true; };
    },
    down(extra = {}) {
      assert.equal(typeof view.__ifSortableStart, 'function', 'The browser pointer gesture must be installed');
      view.__ifSortableStart(Object.assign(event(100, 35), { currentTarget: source }, extra));
    },
    move(x, y, id = 1) { view.emit('pointermove', event(x, y, id)); },
    up(x, y) { view.emit('pointerup', event(x, y)); },
    click() { const e = event(100, 35); view.__ifSortableClick({...e,currentTarget:source, preventDefault(){e.preventDefault();}}); return e; },
    wheel(amount) { const e = event(100, 130); e.deltaY = amount; view.emit('wheel', e); if (!e.defaultPrevented) pane.scrollTop += amount; return e; },
    lastOver() { return events.filter(e => e[0] === 'dragover').at(-1)?.[1]; },
    drops() { return events.filter(e => e[0] === 'drop').map(e => e[1]); },
    key(key, altKey = false) { const e = {...event(0,0),key,altKey}; view.emit("keydown",e); return e; },
    cancel(reason = 'escape') {
      if (reason === 'removed') { source.isConnected = false; frame(32); }
      else if (reason === 'escape') view.emit('keydown', { ...event(0, 0), key: 'Escape' });
      else if (reason === 'lostpointercapture') source.emit(reason, event(0, 0));
      else view.emit(reason, event(0, 0));
    },
    assertClean() {
      assert.equal(frames.size, 0); assert.equal(view.listenerCount(), 0); assert.equal(source.listenerCount(), 0);
      assert.equal(source.captured, null); assert.equal(view.__ifSortableDrag, undefined);
      assert.equal(body.classList.contains('if-sortable-active'), false);
      assert.equal(body.children.some(n => n.classList.contains('if-sortable-ghost')), false);
    },
  };
}
