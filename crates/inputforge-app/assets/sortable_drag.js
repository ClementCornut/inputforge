// Pointer gestures keep normal wheel scrolling available; Rust owns reorder policy.
window.__ifSortableStart ??= function (event) {
    const source = event.currentTarget;
    if (event.button !== 0 || event.isPrimary === false || source.dataset.sortableEnabled !== 'true') return;
    source.__ifSortableMoved = false;
    const doc = source.ownerDocument;
    const view = doc.defaultView;
    view.__ifSortableDrag?.cancel();
    const owner = source.dataset.sortableOwner;
    const context = source.dataset.sortableContext;
    const id = event.pointerId;
    const startX = event.clientX, startY = event.clientY;
    let x = startX, y = startY, active = false, target = null, frame = null, lastTime = null, done = false;
    const handle = source.dataset.sortableHandle;
    const card = source.closest('[data-sortable-row]');
    const origin = card?.getBoundingClientRect();
    let ghost = null;
    let pane = source.parentElement;
    while (pane && !/^(auto|scroll)$/.test(getComputedStyle(pane).overflowY)) pane = pane.parentElement;
    const transfer = new DataTransfer();
    transfer.setData('text/plain', 'inputforge-sortable');
    transfer.effectAllowed = 'move';
    const gesture = { owner, handle: source.dataset.sortableHandle, cancel: () => finish(false) };
    view.__ifSortableDrag = gesture;
    source.setPointerCapture(id);
    event.preventDefault();

    function valid() {
        const group = source.closest('[data-sortable-group]');
        return source.isConnected && source.dataset.sortableEnabled === 'true'
            && source.dataset.sortableOwner === owner && source.dataset.sortableContext === context
            && source.dataset.sortableHandle === handle
            && group?.dataset.sortableGroup === owner && group.dataset.sortableContext === context;
    }

    function emit(node, type) {
        if (node?.isConnected) node.dispatchEvent(new DragEvent(type, {
            bubbles: true, cancelable: true, dataTransfer: transfer, clientX: x, clientY: y,
        }));
    }
    function preview() {
        if (!card) return;
        ghost = card.cloneNode(true);
        // The snapshot must not register as another renderer node, form field or sortable.
        for (const node of [ghost, ...ghost.querySelectorAll('*')]) {
            for (const attr of Array.from(node.attributes)) {
                if (['id', 'name', 'form', 'autofocus', 'data-dioxus-id', 'data-stage-id'].includes(attr.name)
                    || attr.name.startsWith('on') || attr.name.startsWith('data-sortable-')) node.removeAttribute(attr.name);
            }
        }
        ghost.classList.remove('if-sortable--dragging');
        ghost.classList.add('if-sortable-ghost');
        ghost.setAttribute('aria-hidden', 'true');
        ghost.inert = true;
        const rect = card.getBoundingClientRect(), style = getComputedStyle(card);
        Object.assign(ghost.style, { position: 'fixed', top: '0', left: '0', pointerEvents: 'none',
            width: `${rect.width}px`, height: `${rect.height}px`, font: style.font, color: style.color,
            lineHeight: style.lineHeight, letterSpacing: style.letterSpacing });
        const canvases = card.querySelectorAll('canvas');
        ghost.querySelectorAll('canvas').forEach((copy, i) => copy.getContext('2d')?.drawImage(canvases[i], 0, 0));
        doc.body.appendChild(ghost);
    }
    function movePreview() {
        if (ghost) ghost.style.transform = `translate3d(${x - (startX - origin.left)}px, ${y - (startY - origin.top)}px, 0)`;
    }
    function locate() {
        const group = doc.elementFromPoint(x, y)?.closest('[data-sortable-group]');
        if (!group || group.dataset.sortableGroup !== owner || group.dataset.sortableContext !== context) return null;
        const gaps = Array.from(group.children).filter(n => n.matches('.if-sortable-gap') && n.isConnected);
        for (const gap of gaps) {
            const rect = gap.getBoundingClientRect();
            const previous = gap.previousElementSibling, next = gap.nextElementSibling;
            const before = previous?.matches('[data-sortable-row]') ? previous.getBoundingClientRect() : null;
            const after = next?.matches('[data-sortable-row]') ? next.getBoundingClientRect() : null;
            const top = before ? before.top + before.height / 2 : rect.top;
            const bottom = after ? after.top + after.height / 2 : rect.bottom;
            if (x >= rect.left && x <= rect.right && y >= top && y < bottom) return gap;
        }
        // An empty branch includes its Add surface as a drop destination.
        if (gaps.length === 1 && !Array.from(group.children).some(n => n.matches('[data-sortable-row]'))) return gaps[0];
        return null;
    }
    function update() {
        const next = locate();
        if (next === target) return;
        emit(target, 'dragleave');
        target = next;
        emit(target, 'dragover');
    }
    function animate(time) {
        if (done) return;
        if (!valid()) { finish(false); return; }
        if (active) {
            if (pane) {
                const rect = pane.getBoundingClientRect();
                const top = Math.max(0, rect.top + pane.clientTop);
                const bottom = Math.min(view.innerHeight ?? rect.bottom, rect.top + pane.clientTop + pane.clientHeight);
                if (x >= rect.left && x <= rect.right && y >= top && y < bottom) {
                    // A 48px edge band ramps to 600px/s; cap elapsed time to avoid resume jumps.
                    const band = Math.min(48, (bottom - top) / 4);
                    const velocity = y < top + band ? -(1 - (y - top) / band)
                        : y > bottom - band ? 1 - (bottom - y) / band : 0;
                    const elapsed = lastTime === null ? 0 : Math.min(32, time - lastTime);
                    pane.scrollTop = Math.max(0, Math.min(pane.scrollHeight - pane.clientHeight,
                        pane.scrollTop + velocity * 600 * elapsed / 1000));
                }
            }
            update();
        }
        lastTime = time;
        frame = view.requestAnimationFrame(animate);
    }
    function move(e) {
        if (e.pointerId !== id) return;
        if (!valid()) { finish(false); return; }
        if (e.buttons === 0) { finish(false); return; }
        x = e.clientX; y = e.clientY;
        // Four pixels distinguish a deliberate drag from a grip click.
        if (!active && Math.hypot(x - startX, y - startY) >= 4) {
            active = true;
            source.__ifSortableMoved = true;
            preview();
            doc.body.classList.add('if-sortable-active');
            emit(source, 'dragstart');
        }
        if (active) { e.preventDefault(); movePreview(); update(); }
    }
    function up(e) {
        if (e.pointerId !== id) return;
        x = e.clientX; y = e.clientY;
        finish(true);
    }
    function cancel(e) { if (e.pointerId === undefined || e.pointerId === id) finish(false); }
    function key(e) {
        if (e.key === 'Escape') { e.preventDefault(); e.stopImmediatePropagation(); finish(false); }
        else if ((e.altKey && ['ArrowUp', 'ArrowDown'].includes(e.key)) || e.key === 'ContextMenu'
            || (e.shiftKey && e.key === 'F10')) finish(false);
    }
    function finish(deliver) {
        if (done) return;
        deliver = deliver && valid();
        if (active && deliver) update();
        done = true;
        ghost?.remove();
        view.cancelAnimationFrame(frame);
        view.removeEventListener('pointermove', move, true);
        view.removeEventListener('pointerup', up, true);
        view.removeEventListener('pointercancel', cancel, true);
        view.removeEventListener('keydown', key, true);
        view.removeEventListener('blur', cancel, true);
        source.removeEventListener('lostpointercapture', cancel);
        if (source.hasPointerCapture(id)) source.releasePointerCapture(id);
        doc.body.classList.remove('if-sortable-active');
        if (view.__ifSortableDrag === gesture) delete view.__ifSortableDrag;
        if (active) {
            try { if (deliver && source.isConnected) emit(target, 'drop'); }
            finally { emit(source, 'dragend'); }
        }
    }
    view.addEventListener('pointermove', move, true);
    view.addEventListener('pointerup', up, true);
    view.addEventListener('pointercancel', cancel, true);
    view.addEventListener('keydown', key, true);
    view.addEventListener('blur', cancel, true);
    source.addEventListener('lostpointercapture', cancel);
    frame = view.requestAnimationFrame(animate);
};

window.__ifSortableClick ??= function(event) {
    const source = event.currentTarget;
    if (source.__ifSortableMoved) {
        source.__ifSortableMoved = false;
        event.preventDefault(); event.stopPropagation();
    }
};
