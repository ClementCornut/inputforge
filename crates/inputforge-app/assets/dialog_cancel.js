const key = __KEY__;
window[key]?.();
const dialog = document.getElementById(__ID__);
const cancel = event => event.preventDefault();
// Controlled dialogs close only after the app acknowledges dismissal.
dialog?.addEventListener('cancel', cancel);
let disposed = false;
window[key] = () => {
    if (disposed) return;
    disposed = true;
    dialog?.removeEventListener('cancel', cancel);
};
