const listenerKey = __KEY__;
window[listenerKey]?.();
const handler = event => {
    const target = document.getElementById(__TARGET__);
    if (!target) return;
    const path = event.composedPath?.();
    const inside = node => node && (path ? path.includes(node) : node.contains(event.target));
    if (inside(target) || inside(document.getElementById(__EXCLUDE__))) return;
    dioxus.send('away');
};
document.addEventListener('mousedown', handler);
let disposed = false;
window[listenerKey] = () => {
    if (disposed) return;
    disposed = true;
    document.removeEventListener('mousedown', handler);
};
(async () => {
    while (await dioxus.recv() !== '__shutdown__') {}
    window[listenerKey]?.(); delete window[listenerKey];
})();
