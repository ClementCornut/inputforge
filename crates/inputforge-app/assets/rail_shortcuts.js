const h = (ev) => {
       if (window.__ifSortableDrag) return;
       if (ev.defaultPrevented || document.querySelector('dialog[open], [role="menu"], [role="dialog"]')) return;
       const editableSelector = 'input, textarea, select, [contenteditable], [role="textbox"]';
       const active = document.activeElement;
       if (active?.closest('.if-editor') && !((ev.ctrlKey || ev.metaKey) && ['f','F'].includes(ev.key))) return;
       const filterEscape = ev.key === 'Escape' && active?.closest('.if-rail__filter');
       if (!filterEscape && active && active.closest(editableSelector)) return;
       const rawTarget = ev.target;
       const target = rawTarget instanceof Element ? rawTarget : rawTarget?.parentElement;
       if (!filterEscape && target && target.closest(editableSelector)) return;
       const meta = ev.metaKey ? 1 : 0;
       const ctrl = ev.ctrlKey ? 1 : 0;
       const alt  = ev.altKey  ? 1 : 0;
       dioxus.send([ev.key, meta, ctrl, alt, document.querySelector('.if-rail')?.dataset.navigationContext ?? '']);
     };
     window['__LISTENER_KEY__']?.();
     window.addEventListener('keydown', h, true);
     window['__LISTENER_KEY__'] = () => window.removeEventListener('keydown', h, true);
     (async () => {
       while (true) {
         const msg = await dioxus.recv();
         if (msg === '__shutdown__') {
           window.removeEventListener('keydown', h, true);
           dioxus.send(['__ack__', 0, 0, 0]);
           return;
         }
       }
     })();
     