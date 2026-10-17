// Execute the production gesture against controlled DOM geometry and frame timing.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture } from './sortable_pointer/fixture.mjs';
test('both card halves and their gap route to the same insertion slot', () => {
  for (const y of [130, 160, 180]) {
    const f = fixture(); f.down(); f.move(100, y);
    assert.equal(f.lastOver(), 'gap-2'); f.up(100, y);
    assert.deepEqual(f.drops(), ['gap-2']); f.assertClean();
  }
});
test('leading, trailing and midpoint boundaries use measured card heights', () => {
  for (const [y, want] of [[25, 'gap-0'], [47, 'gap-0'], [48, 'gap-1'], [115, 'gap-1'], [116, 'gap-2'], [193, 'gap-2'], [194, 'gap-3'], [230, 'gap-3']]) {
    const f = fixture(); f.down(); f.move(100, y);
    assert.equal(f.lastOver(), want); f.cancel(); f.assertClean();
  }
});
test('wheel scrolling remains available and refreshes a stationary target', () => {
  const f = fixture(); f.down(); f.move(100, 130);
  assert.equal(f.lastOver(), 'gap-2');
  const wheel = f.wheel(80);
  assert.equal(wheel.defaultPrevented, false); assert.equal(f.pane.scrollTop, 80);
  f.frame(16); assert.equal(f.lastOver(), 'gap-3');
  f.up(100, 130); assert.deepEqual(f.drops(), ['gap-3']); f.assertClean();
});
test('edge scrolling moves only the containing pane in both directions', () => {
  for (const [y, sign] of [[5, -1], [155, 1]]) {
    const f = fixture(160); f.pane.scrollTop = 80; f.down(); f.move(100, y);
    f.frame(16); f.frame(32); assert.equal(Math.sign(f.pane.scrollTop - 80), sign);
    f.cancel(); const position = f.pane.scrollTop; f.frame(48);
    assert.equal(f.pane.scrollTop, position); f.assertClean();
  }
});
test('scroll speed increases toward an edge and stops outside the pane', () => {
  const distances = [];
  for (const y of [125, 155]) {
    const f = fixture(160); f.down(); f.move(100, y); f.frame(16); f.frame(32);
    distances.push(f.pane.scrollTop); f.cancel();
  }
  assert.ok(distances[1] > distances[0]);
  const f = fixture(160); f.down(); f.move(400, 155); f.frame(16); f.frame(32);
  assert.equal(f.pane.scrollTop, 0); f.up(400, 155);
  assert.deepEqual(f.drops(), []); f.assertClean();
});
test('Escape, cancellation, capture loss, blur and source removal release resources', () => {
  for (const reason of ['escape', 'pointercancel', 'lostpointercapture', 'blur', 'removed']) {
    const f = fixture(); f.down(); f.move(100, 180); f.cancel(reason);
    assert.deepEqual(f.drops(), []); f.assertClean(); f.source.isConnected = true;
    f.down(); f.move(100, 180); f.up(100, 180);
    assert.deepEqual(f.drops(), ['gap-2']); f.assertClean();
  }
});
test('clicks, disabled handles, secondary buttons and unrelated pointers never reorder', () => {
  const f = fixture(); f.down(); f.up(100, 35);
  assert.deepEqual(f.events, []); f.assertClean();
  f.source.dataset.sortableEnabled = 'false'; f.down(); f.move(100, 180); f.up(100, 180);
  assert.deepEqual(f.events, []); f.assertClean();
  f.source.dataset.sortableEnabled = 'true'; f.down({ button: 2 }); f.move(100, 180);
  assert.deepEqual(f.events, []); f.assertClean();
  f.down(); f.move(100, 180, 9); assert.deepEqual(f.events, []);
  f.cancel(); f.assertClean();
});
test('foreign list owners and newly removed targets do not receive a drop', () => {
  const f = fixture(); f.down(); f.move(100, 180);
  f.group.dataset.sortableGroup = 'different-list'; f.up(100, 180);
  assert.deepEqual(f.drops(), []); f.assertClean();
  const g = fixture(); g.down(); g.move(100, 180); g.gaps[2].isConnected = false;
  g.up(100, 180); assert.deepEqual(g.drops(), []); g.assertClean();
});
test('a drag suppresses its release click but an ordinary grip click still selects', () => {
  const f = fixture(); f.down(); f.move(100, 180); f.up(100, 180);
  assert.equal(f.click().defaultPrevented, true);
  f.down(); f.up(100, 35); assert.equal(f.click().defaultPrevented, false);
  f.assertClean();
});
test('nested empty branches own the target instead of their outer card', () => {
  const f = fixture(); f.emptyBranch(); f.down(); f.move(100, 130);
  assert.equal(f.lastOver(), 'branch-gap'); f.up(100, 130);
  assert.deepEqual(f.drops(), ['branch-gap']); f.assertClean();
});
test('a reused source or destination cancels when its prepared context changes', () => {
  for (const [node, phase] of [['source', 'frame'], ['source', 'release'], ['source', 'move'], ['group', 'release']]) {
    const f = fixture(); f.down(); f.move(100, 180);
    f[node].dataset.sortableContext = 'different-profile-mode-mapping-or-tree';
    if (phase === 'frame') f.frame(16);
    if (phase === 'move') f.move(100, 130);
    f.up(100, 180);
    assert.deepEqual(f.drops(), []); f.assertClean();
  }
});
test('a full card ghost preserves its grab offset while following the pointer', () => {
  const f = fixture(); f.down();
  assert.equal(f.ghost(), undefined);
  f.move(100, 130);
  const ghost = f.ghost();
  assert.ok(ghost, 'A pointer drag must show the card preview');
  assert.equal(ghost.textContent, 'Mapping card summary');
  assert.equal(ghost.style.width, '300px'); assert.equal(ghost.style.height, '40px');
  assert.equal(ghost.style.transform, 'translate3d(0px, 123px, 0)');
  f.move(140, 180);
  assert.equal(ghost.style.transform, 'translate3d(40px, 173px, 0)');
  f.wheel(30); f.frame(16);
  assert.equal(ghost.style.transform, 'translate3d(40px, 173px, 0)');
  f.up(140, 180); f.assertClean();
});
test('the visual ghost cannot become a drop target, focus target or renderer node', () => {
  const f = fixture(); f.down(); f.move(100, 180);
  const ghost = f.ghost(); assert.ok(ghost);
  assert.equal(ghost.inert, true); assert.equal(ghost.style.pointerEvents, 'none');
  assert.equal(ghost.extraAttributes.get('aria-hidden'), 'true');
  for (const n of [ghost, ...ghost.querySelectorAll('*')]) {
    assert.equal(n.id, '');
    assert.equal(n.extraAttributes.has('data-dioxus-id'), false);
    assert.equal(n.extraAttributes.has('onpointerdown'), false);
    assert.equal(n.extraAttributes.has('name'), false);
    assert.equal(n.extraAttributes.has('autofocus'), false);
    assert.ok(!Object.keys(n.dataset).some(key => key.startsWith('sortable')));
    assert.equal(n.dataset.stageId, undefined);
  }
  assert.equal(f.source.id, 'handle');
  assert.equal(f.source.extraAttributes.get('data-dioxus-id'), 'native-handle');
  assert.equal(f.lastOver(), 'gap-2'); f.up(100, 180);
  assert.deepEqual(f.drops(), ['gap-2']); f.assertClean();
});
test('expanded card previews retain their canvas drawing', () => {
  const f = fixture(); const canvas = f.canvas(); f.down(); f.move(100, 180);
  const ghost = f.ghost(); assert.ok(ghost);
  assert.deepEqual(ghost.querySelectorAll('canvas')[0].draws, [[canvas, 0, 0]]);
  f.cancel(); f.assertClean();
});

test('reusing the mounted handle identity cancels before delivery', () => {
  const f = fixture(); f.source.dataset.sortableHandle = 'first'; f.down(); f.move(100, 180);
  f.source.dataset.sortableHandle = 'replacement'; f.up(100, 180);
  assert.deepEqual(f.drops(), []); f.assertClean();
});
test('replacing a source group cancels even when the handle context remains unchanged', () => {
  const f = fixture(); f.down(); f.move(100, 180);
  f.group.dataset.sortableContext = 'replacement-profile-mode-tree'; f.frame(16);
  assert.deepEqual(f.drops(), []); f.assertClean();
});
test('a second gesture cancels the old owner and delivers only once', () => {
  const f = fixture(); f.down(); f.move(100, 180); f.down(); f.move(100, 180);
  f.up(100, 180); f.up(100, 180);
  assert.deepEqual(f.drops(), ['gap-2']); f.assertClean();
});

test('keyboard reorder cancels the pointer gesture without consuming the keyboard action', () => {
  const f = fixture(); f.down(); f.move(100,180);
  const e = f.key('ArrowDown', true);
  assert.equal(e.defaultPrevented,false); f.assertClean();
  f.up(100,180); assert.deepEqual(f.drops(),[]);
});
test('moving a mounted source into a different owner cancels before the next frame', () => {
  const f = fixture(); f.down(); f.move(100,180);
  f.group.dataset.sortableGroup = 'replacement-owner'; f.frame(16);
  assert.deepEqual(f.drops(),[]); f.assertClean();
});
