CREATE TABLE IF NOT EXISTS campaigns (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    link TEXT,
    is_collab BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS maps (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    -- FK lives here (the "many" side). NULL means standalone — no separate
    -- is_standalone column, since that would just duplicate this check.
    campaign_id INTEGER REFERENCES campaigns(id) ON DELETE SET NULL,
    nb_rooms INTEGER,
    cleared BOOLEAN NOT NULL DEFAULT FALSE,
    clearing_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
