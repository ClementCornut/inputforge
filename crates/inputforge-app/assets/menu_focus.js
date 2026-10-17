function(menuId, action) {
    const menu = document.getElementById(menuId);
    if (!menu) return;
    const items = Array.from(menu.querySelectorAll('[role="menuitem"]'))
        .filter(item => !item.disabled && item.getAttribute('aria-disabled') !== 'true');
    if (!items.length) return;
    const index = items.indexOf(document.activeElement);
    let target;
    if (action === 'down') target = items[(index + 1) % items.length];
    else if (action === 'up') target = items[index <= 0 ? items.length - 1 : index - 1];
    else if (action === 'first') target = items[0];
    else if (action === 'last') target = items[items.length - 1];
    target?.focus();
}
