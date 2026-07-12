-- Login sessions for the shared-secret auth (/login?key=...). Only the
-- SHA-256 of the browser token is stored — the raw value lives in the
-- cookie alone, so a leaked table can't be replayed as a session.
CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
