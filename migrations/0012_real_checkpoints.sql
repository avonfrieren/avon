-- Two checkpoint flags per room: checkpoint_start is the working state —
-- freely toggled in the dashboard to measure any segment — and
-- checkpoint_real is the map's actual checkpoints, the persistent
-- reference that "save as real" overwrites and "reset" restores.
ALTER TABLE rooms ADD COLUMN checkpoint_real BOOLEAN NOT NULL DEFAULT FALSE;

-- Whatever is currently flagged becomes the initial reference.
UPDATE rooms SET checkpoint_real = checkpoint_start;
