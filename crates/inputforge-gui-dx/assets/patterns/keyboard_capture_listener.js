window.__inputforgeKeyboardCaptureArmed = false;
const sendCapture = (ev, kind) => {
  if (!window.__inputforgeKeyboardCaptureArmed) return;
  ev.preventDefault();
  ev.stopPropagation();
  const payload = {
    kind,
    key: ev.key || '',
    code: ev.code || 'Unidentified',
    location: ev.location || 0,
    ctrl: !!ev.ctrlKey,
    alt: !!ev.altKey,
    shift: !!ev.shiftKey,
    meta: !!ev.metaKey,
  };
  console.debug('[keyboard_capture]', payload);
  dioxus.send(payload);
};
const keydown = (ev) => sendCapture(ev, 'keydown');
const keyup = (ev) => sendCapture(ev, 'keyup');
window.addEventListener('keydown', keydown, true);
window.addEventListener('keyup', keyup, true);
(async () => {
  while (true) {
    const msg = await dioxus.recv();
    if (msg === '__shutdown__') {
      window.removeEventListener('keydown', keydown, true);
      window.removeEventListener('keyup', keyup, true);
      dioxus.send('__ack__');
      return;
    }
  }
})();
