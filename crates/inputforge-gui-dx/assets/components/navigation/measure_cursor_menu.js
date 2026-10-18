function (id) {
    requestAnimationFrame(() => {
        const menu = document.getElementById(id);
        if (!menu) {
            dioxus.send([0, 0, 0, 0]);
            return;
        }
        const r = menu.getBoundingClientRect();
        dioxus.send([r.width, r.height, window.innerWidth, window.innerHeight]);
    });
}
