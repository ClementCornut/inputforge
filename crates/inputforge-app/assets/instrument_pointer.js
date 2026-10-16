// Desktop plot transport. Geometry is measured live and interpreted by the app.
const plotId = __PLOT_ID__;
const owner = __OWNER__;
const plot = document.getElementById(plotId);
if (!plot) return;
const registry = window.__inputforgeInstruments ??= new Map();
registry.get(owner)?.();
let dragging = false;
let alive = true;
const listeners = [];
const on = (target, kind, handler) => {
    target.addEventListener(kind, handler);
    listeners.push(() => target.removeEventListener(kind, handler));
};
const cleanup = () => {
    if (!alive) return;
    alive = false;
    dragging = false;
    for (const remove of listeners) remove();
    if (registry.get(owner) === cleanup) registry.delete(owner);
};
registry.set(owner, cleanup);
const current = () => alive && document.getElementById(plotId) === plot;
const send = (kind, event) => {
    if (!current()) { cleanup(); return; }
    const rect = plot.getBoundingClientRect();
    dioxus.send({ kind, x: event?.clientX | 0, y: event?.clientY | 0,
        rl: rect.left, rt: rect.top, rs: Math.min(rect.width, rect.height) });
};
const cancel = () => {
    if (!dragging) return;
    dragging = false;
    send('cancel');
};
on(plot, 'mousedown', event => {
    if (event.button !== 0 || !current()) return;
    dragging = true;
    send('down', event);
});
on(document, 'mousemove', event => {
    if (!current()) { cleanup(); return; }
    const rect = plot.getBoundingClientRect();
    if (!dragging && !(event.clientX >= rect.left && event.clientX <= rect.right &&
        event.clientY >= rect.top && event.clientY <= rect.bottom)) return;
    send('move', event);
});
on(document, 'mouseup', event => {
    if (!current()) { cleanup(); return; }
    if (event.button !== 0 || !dragging) return;
    dragging = false;
    send('up', event);
});
on(plot, 'dblclick', event => send('dbl', event));
on(plot, 'contextmenu', event => {
    event.preventDefault();
    if (dragging) { dragging = false; send('up', event); }
    send('ctx', event);
});
on(window, 'blur', cancel);
on(document, 'pointercancel', cancel);
on(plot, 'lostpointercapture', cancel);
on(document, 'keydown', event => {
    if (event.key === 'Escape') cancel();
});
(async () => {
    while (await dioxus.recv() !== '__shutdown__') {}
    cleanup();
    dioxus.send('shutdown_ack');
})();
