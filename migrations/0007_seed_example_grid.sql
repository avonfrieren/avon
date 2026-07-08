-- Example data for the grid: one campaign, one linked map, 8 placeholder
-- rooms, the 25 challenge codes provided so far (the 26th is still to be
-- identified), and deterministic placeholder times in most cells — a few
-- are left empty on purpose to show the empty-cell rendering.
WITH camp AS (
    INSERT INTO campaigns (name, is_collab)
    VALUES ('Example Campaign', false)
    RETURNING id
), m AS (
    INSERT INTO maps (name, campaign_id, nb_rooms)
    SELECT 'Example Map', id, 8 FROM camp
    RETURNING id
), r AS (
    INSERT INTO rooms (map_id, name, position)
    SELECT m.id, 'r' || lpad(i::text, 2, '0'), i
    FROM m, generate_series(1, 8) AS i
    RETURNING id, position
), c AS (
    INSERT INTO challenges (map_id, name, position)
    SELECT m.id, v.name, v.position
    FROM m, (VALUES
        ('t', 1), ('dc', 2), ('g', 3), ('j', 4), ('f', 5), ('fr', 6),
        ('dd', 7), ('to', 8), ('b', 9), ('u', 10), ('hs', 11), ('ds', 12),
        ('og', 13), ('hg', 14), ('dg', 15), ('s', 16), ('is', 17),
        ('fz', 18), ('if', 19), ('di', 20), ('h', 21), ('1x', 22),
        ('2x', 23), ('3x', 24), ('4x', 25)
    ) AS v(name, position)
    RETURNING id, position
)
INSERT INTO challenge_times (room_id, challenge_id, time_ms)
SELECT r.id, c.id,
       4000 + r.position * 1300 + c.position * 450
            + (r.position * c.position * 137) % 3500
FROM r, c
WHERE (r.position * 7 + c.position) % 11 <> 0;
