const listenerKey = '__LISTENER_KEY__';
window[listenerKey]?.();
const handler = event => {
    if (event.key === 'Escape' && !event.target?.closest?.('.if-rail__filter')) dioxus.send('esc');
};
window.addEventListener('keydown', handler, true);
let disposed = false;
window[listenerKey] = () => {
    if (disposed) return;
    disposed = true;
    window.removeEventListener('keydown', handler, true);
};
(async () => {
    while (await dioxus.recv() !== '__shutdown__') {}
    window[listenerKey]?.(); delete window[listenerKey];
    dioxus.send('shutdown_ack');
})();
