-- Checkpoint semantics flip: a flagged room now ENDS its segment
-- (included) instead of starting the next one — checking a room includes
-- it. Renamed accordingly, and existing flags shift one room up (the old
-- "starts at 7" becomes "ends at 6") so every map's current grouping is
-- preserved under the new meaning. A flag on a map's first room had no
-- effect before and drops out naturally (LEAD's NULL -> FALSE).
ALTER TABLE rooms RENAME COLUMN checkpoint_start TO checkpoint_end;

WITH shifted AS (
    SELECT id,
           LEAD(checkpoint_end)  OVER w AS next_end,
           LEAD(checkpoint_real) OVER w AS next_real
    FROM rooms
    WINDOW w AS (PARTITION BY map_id ORDER BY position, id)
)
UPDATE rooms r
SET checkpoint_end  = COALESCE(s.next_end, FALSE),
    checkpoint_real = COALESCE(s.next_real, FALSE)
FROM shifted s
WHERE s.id = r.id;
