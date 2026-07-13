-- Per-room difficulty value for a bool challenge's dashboard. The bool
-- itself lives in challenge_values (0/1); this is the separate "diff"
-- integer entered by hand next to it. The map's difficulty is the sum of
-- these over its rooms, divided by the room count (same per checkpoint).
CREATE TABLE IF NOT EXISTS challenge_diffs (
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    diff INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (challenge_id, room_id)
);
