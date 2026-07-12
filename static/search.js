// Ctrl+K search palette. No server round-trip: maps, campaigns and imgs
// are all already in the sidebar DOM, so the palette just indexes those
// links each time it opens (which also keeps it in sync after renames,
// creations and deletions).
//
// Keys: Ctrl+K open/close, type to filter, Up/Down to move, Enter or
// Ctrl+J to open the selection, Escape closes. (Plain "j" can't confirm:
// it has to stay typable in the search field.)
(function () {
    const overlay = document.createElement('div');
    overlay.id = 'search-overlay';
    overlay.innerHTML =
        '<div id="search-box">' +
        '<input id="search-input" placeholder="search maps, campaigns, imgs...">' +
        '<ul id="search-results"></ul>' +
        '</div>';
    document.body.appendChild(overlay);

    const input = overlay.querySelector('#search-input');
    const list = overlay.querySelector('#search-results');
    const MAX_RESULTS = 10;
    let items = [];
    let filtered = [];
    let selected = 0;

    function collect() {
        items = [];
        document.querySelectorAll('.sidebar a[hx-get^="/map/"]').forEach(function (a) {
            items.push({ type: 'map', name: a.textContent.trim(), el: a });
        });
        // Campaign folders are the nested <details> inside the campaigns one.
        document.querySelectorAll('.sidebar details details > summary').forEach(function (s) {
            items.push({ type: 'campaign', name: s.textContent.trim(), el: s });
        });
        document.querySelectorAll('.sidebar a[hx-get^="/docs/"]').forEach(function (a) {
            items.push({ type: 'doc', name: a.textContent.trim(), el: a });
        });
        document.querySelectorAll('.sidebar a[hx-get^="/imgs/"]').forEach(function (a) {
            items.push({ type: 'img', name: a.textContent.trim(), el: a });
        });
    }

    function render() {
        list.innerHTML = '';
        filtered.slice(0, MAX_RESULTS).forEach(function (item, i) {
            const li = document.createElement('li');
            li.textContent = item.name;
            const tag = document.createElement('span');
            tag.className = 'search-tag';
            tag.textContent = item.type;
            li.appendChild(tag);
            if (i === selected) li.classList.add('selected');
            li.addEventListener('click', function () { activate(item); });
            list.appendChild(li);
        });
    }

    function filter() {
        const q = input.value.trim().toLowerCase();
        filtered = q
            ? items.filter(function (it) { return it.name.toLowerCase().includes(q); })
            : items.slice();
        selected = 0;
        render();
    }

    function open() {
        collect();
        input.value = '';
        filter();
        overlay.classList.add('open');
        input.focus();
    }

    function close() {
        overlay.classList.remove('open');
    }

    function activate(item) {
        close();
        if (item.type === 'campaign') {
            item.el.parentElement.open = true;
            item.el.scrollIntoView({ block: 'nearest' });
        } else {
            // Unfold the folders above it, then let HTMX handle the click.
            let d = item.el.closest('details');
            while (d) {
                d.open = true;
                d = d.parentElement ? d.parentElement.closest('details') : null;
            }
            item.el.click();
        }
    }

    document.addEventListener('keydown', function (e) {
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
            e.preventDefault();
            overlay.classList.contains('open') ? close() : open();
            return;
        }
        if (!overlay.classList.contains('open')) return;

        if (e.key === 'Escape') {
            close();
        } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            selected = Math.min(selected + 1, Math.min(filtered.length, MAX_RESULTS) - 1);
            render();
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            selected = Math.max(selected - 1, 0);
            render();
        } else if (e.key === 'Enter' || (e.ctrlKey && e.key.toLowerCase() === 'j')) {
            e.preventDefault();
            if (filtered[selected]) activate(filtered[selected]);
        }
    });

    input.addEventListener('input', filter);
    overlay.addEventListener('mousedown', function (e) {
        if (e.target === overlay) close();
    });
})();
