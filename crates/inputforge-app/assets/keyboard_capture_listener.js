// One passive root listener retains registration order across capture requests.
const listenerKey = '__LISTENER_KEY__';
let serial = null;
const sendCapture = (ev, kind) => {
  if (serial === null) return;
  ev.preventDefault();
  ev.stopPropagation();
  ev.stopImmediatePropagation();
  dioxus.send({
    serial,
    kind,
    key: ev.key || '',
    code: ev.code || 'Unidentified',
    location: ev.location || 0,
    ctrl: !!ev.ctrlKey,
    alt: !!ev.altKey,
    shift: !!ev.shiftKey,
    meta: !!ev.metaKey,
  });
};
const keydown = ev => sendCapture(ev, 'keydown');
const keyup = ev => sendCapture(ev, 'keyup');
window.addEventListener('keydown', keydown, true);
window.addEventListener('keyup', keyup, true);
const cleanup = () => {
  serial = null;
  window.removeEventListener('keydown', keydown, true);
  window.removeEventListener('keyup', keyup, true);
};
cleanup.arm = value => { serial = value; };
cleanup.disarm = () => { serial = null; };
window[listenerKey] = cleanup;
(async () => {
  while (await dioxus.recv() !== '__shutdown__') {}
  window[listenerKey]?.(); delete window[listenerKey];
  dioxus.send('shutdown_ack');
})();
