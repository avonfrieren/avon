CREATE TABLE IF NOT EXISTS challenges (
    id SERIAL PRIMARY KEY,
    -- CASCADE: a challenge is meaningless without its map, so deleting the
    -- map takes its challenge list with it.
    map_id INTEGER NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- Best time in milliseconds. NULL = not timed yet; NULL rows are simply
    -- left out of the Sum of Best.
    best_time_ms BIGINT,
    -- Display order in the map's challenge table (challenges have a fixed
    -- in-game order, so alphabetical sorting would scramble them).
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
