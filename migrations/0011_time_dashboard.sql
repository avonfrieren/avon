-- Time dashboard groundwork.
--
-- Checkpoints: a room flagged checkpoint_start opens a new segment group
-- (the first room implicitly opens the first one). Storing the start
-- room rather than a rooms-per-checkpoint count means inserting or
-- deleting rooms can't silently shift every later checkpoint.
ALTER TABLE rooms ADD COLUMN checkpoint_start BOOLEAN NOT NULL DEFAULT FALSE;

-- The best complete run for a time challenge, stored as the cumulative
-- split at each room (the mod's "Best Split" column). Segments are
-- derived by difference, and the last room's split is the run's total —
-- which is what decides whether an imported run replaces this one.
CREATE TABLE IF NOT EXISTS run_splits (
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    split_ms BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (challenge_id, room_id)
);
