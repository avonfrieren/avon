-- The first challenges design (0004: one flat list of best times per map)
-- didn't match the real data, which is a 2D grid — rooms on the vertical
-- axis, challenges on the horizontal one, a time in each cell. Rebuild
-- storage around that: rooms and challenges are the two axes, and a cell
-- table holds the times, keyed by (room, challenge).

DROP TABLE IF EXISTS challenges;

-- The 0005 example data was shaped for the old design; the grid seed
-- (0007) recreates it. Map first: deleting the campaign first would just
-- SET NULL the map's FK and leave it behind as a standalone map.
DELETE FROM maps WHERE name = 'Example Map';
DELETE FROM campaigns WHERE name = 'Example Campaign';

CREATE TABLE IF NOT EXISTS rooms (
    id SERIAL PRIMARY KEY,
    map_id INTEGER NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- Rooms have a fixed in-game order; alphabetical would scramble them.
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS challenges (
    id SERIAL PRIMARY KEY,
    map_id INTEGER NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One filled-in cell of the grid. An empty cell is simply a missing row,
-- so time_ms itself is NOT NULL — clearing a cell deletes the row.
CREATE TABLE IF NOT EXISTS challenge_times (
    room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    time_ms BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (room_id, challenge_id)
);
