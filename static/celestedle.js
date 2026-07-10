// Celestedle guess palette. Same look and keys as the site's Ctrl+K
// search (the CSS palette classes are shared), but it lists the game's
// entities and submits the pick as a guess via htmx. While the game pane
// is open, Ctrl+K opens this palette instead of the site search — the
// capture-phase listener runs first and stops the search.js one.
(function () {
    const overlay = document.createElement('div');
    overlay.className = 'palette-overlay';
    overlay.innerHTML =
        '<div class="palette-box">' +
        '<input class="palette-input" placeholder="guess an entity...">' +
        '<ul class="palette-results"></ul>' +
        '</div>';
    document.body.appendChild(overlay);

    const input = overlay.querySelector('.palette-input');
    const list = overlay.querySelector('.palette-results');
    const MAX_RESULTS = 10;
    let entities = [];
    const guessed = new Set();
    let done = false;
    let filtered = [];
    let selected = 0;

    const gameOpen = () => !!document.getElementById('celestedle');

    function collect() {
        const data = document.getElementById('celestedle-entities');
        entities = data ? JSON.parse(data.textContent) : [];
    }

    function render() {
        list.innerHTML = '';
        filtered.slice(0, MAX_RESULTS).forEach(function (e, i) {
            const li = document.createElement('li');
            li.textContent = e.name;
            if (i === selected) li.classList.add('selected');
            li.addEventListener('click', function () { submit(e); });
            list.appendChild(li);
        });
    }

    function filter() {
        const q = input.value.trim().toLowerCase();
        filtered = entities.filter(function (e) {
            return !guessed.has(e.id) && (!q || e.name.toLowerCase().includes(q));
        });
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

    function submit(entity) {
        close();
        if (done) return;
        guessed.add(entity.id);
        htmx.ajax('POST', '/celestedle/guess', {
            target: '#celestedle-rows',
            swap: 'afterbegin',
            values: { id: entity.id },
        }).then(function () {
            if (document.querySelector('#celestedle-rows tr.win')) {
                done = true;
                const status = document.getElementById('celestedle-status');
                const n = guessed.size;
                status.textContent =
                    '✓ found in ' + n + (n === 1 ? ' guess' : ' guesses') +
                    ' — come back tomorrow';
            }
        });
    }

    // Each fresh game pane is a fresh game (client-side state only).
    document.body.addEventListener('htmx:afterSwap', function (e) {
        if (e.target.id === 'content' && gameOpen()) {
            guessed.clear();
            done = false;
        }
    });

    document.addEventListener('click', function (e) {
        if (e.target.id === 'celestedle-open') open();
    });

    document.addEventListener('keydown', function (e) {
        if (!gameOpen()) return;
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
            e.preventDefault();
            e.stopImmediatePropagation(); // keep search.js out of it
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
            if (filtered[selected]) submit(filtered[selected]);
        }
    }, true);

    input.addEventListener('input', filter);
    overlay.addEventListener('mousedown', function (e) {
        if (e.target === overlay) close();
    });
})();
