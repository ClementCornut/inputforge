// App-owned snapshot shortcut, installed after the root keyboard interceptor.
const listenerKey = '__LISTENER_KEY__';
let generation = __GENERATION__;
let pickerReturn = false;
const keydown = ev => {
  const save = (ev.key === 's' || ev.key === 'S') && (ev.ctrlKey || ev.metaKey);
  if (!save) {
    if (!['Control', 'Meta', 'Alt', 'Shift', 'AltGraph'].includes(ev.key)) pickerReturn = false;
    return;
  }
  if (ev.defaultPrevented || ev.repeat || ev.isComposing || ev.shiftKey || ev.altKey || pickerReturn) return;
  const ae = document.activeElement;
  const tag = ae?.tagName?.toUpperCase();
  if (['INPUT', 'TEXTAREA', 'SELECT'].includes(tag) || ae?.isContentEditable) return;
  if (ae?.closest?.('.profile-row__rename') || ae?.closest?.('[data-inline-rename]')) return;
  if (ae?.closest?.('[role="menu"]:not([aria-hidden="true"])') || document.querySelector('[role="menu"]:not([aria-hidden="true"])')) return;
  if (ae?.closest?.('[role="dialog"][aria-modal="true"]') || document.querySelector('dialog[open]')) return;
  ev.preventDefault();
  dioxus.send(generation);
};
const blur = ev => { if (ev.target === window) pickerReturn = true; };
const pointerdown = () => { pickerReturn = false; };
window.addEventListener('keydown', keydown, true);
window.addEventListener('blur', blur, true);
window.addEventListener('pointerdown', pointerdown, true);
const cleanup = () => {
  window.removeEventListener('keydown', keydown, true);
  window.removeEventListener('blur', blur, true);
  window.removeEventListener('pointerdown', pointerdown, true);
};
cleanup.arm = value => { generation = value; };
window[listenerKey] = cleanup;
(async () => {
  while (await dioxus.recv() !== '__shutdown__') {}
  window[listenerKey]?.(); delete window[listenerKey];
  dioxus.send('shutdown_ack');
})();
