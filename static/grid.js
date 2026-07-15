// Keyboard navigation for the challenge grid's editable cells.
//
// - Enter (text cell): saves and moves to the cell BELOW (next in the
//   column, not across).
// - Arrows: move between cells — Up/Down always, Left/Right only when the
//   text caret is already at the edge, so normal in-field editing still
//   works.
// - Space still toggles a checkbox.
//
// Every edit re-renders the whole grid (to keep the SoB footer in sync),
// which replaces the inputs and drops focus. So a move stashes where
// focus should land, and htmx:afterSwap restores it once the fresh grid
// is in place. Cells are addressed by their data-row / data-col indices,
// which are stable for a given map.
(function () {
    let pendingFocus = null; // {row, col} to focus after the next grid swap

    function cellAt(row, col) {
        return document.querySelector(
            '#challenge-grid input[data-row="' + row + '"][data-col="' + col + '"]'
        );
    }

    function focusCell(row, col) {
        const t = cellAt(row, col);
        if (t) {
            t.focus();
            if (t.select) t.select();
        }
        return t;
    }

    document.addEventListener('keydown', function (e) {
        const el = e.target;
        if (
            !el.matches ||
            !el.matches('#challenge-grid input.cell, #challenge-grid input.cell-check')
        ) {
            return;
        }
        const row = parseInt(el.dataset.row, 10);
        const col = parseInt(el.dataset.col, 10);
        const isText = el.classList.contains('cell');

        if (e.key === 'Enter') {
            e.preventDefault();
            pendingFocus = { row: row + 1, col: col };
            if (isText) {
                // Force the save; the grid re-renders and afterSwap lands
                // focus on the cell below.
                el.dispatchEvent(new Event('change', { bubbles: true }));
            } else {
                focusCell(row + 1, col); // checkbox: Enter just moves down
            }
        } else if (e.key === ' ' && !isText) {
            // Let space toggle the box; keep focus on it after the re-render.
            pendingFocus = { row: row, col: col };
        } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            pendingFocus = { row: row + 1, col: col };
            focusCell(row + 1, col);
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            pendingFocus = { row: row - 1, col: col };
            focusCell(row - 1, col);
        } else if (e.key === 'ArrowRight') {
            if (isText && el.selectionStart !== el.value.length) return;
            e.preventDefault();
            pendingFocus = { row: row, col: col + 1 };
            focusCell(row, col + 1);
        } else if (e.key === 'ArrowLeft') {
            if (isText && el.selectionStart !== 0) return;
            e.preventDefault();
            pendingFocus = { row: row, col: col - 1 };
            focusCell(row, col - 1);
        }
    });

    // A mouse interaction cancels any pending keyboard focus intent.
    document.addEventListener('pointerdown', function () {
        pendingFocus = null;
    });

    // After the grid re-renders from a save, put focus back where the
    // keyboard move meant it to go.
    document.body.addEventListener('htmx:afterSwap', function () {
        if (pendingFocus && document.getElementById('challenge-grid')) {
            focusCell(pendingFocus.row, pendingFocus.col);
            pendingFocus = null;
        }
    });
})();
