-- A second example, this time standalone (campaign_id NULL), with the same
-- placeholder data as 0007 — same 8 rooms, same challenge set and kinds,
-- same deterministic values (the 0008 kind rescaling applied inline).
WITH m AS (
    INSERT INTO maps (name, campaign_id, nb_rooms)
    VALUES ('Standalone Example', NULL, 8)
    RETURNING id
), r AS (
    INSERT INTO rooms (map_id, name, position)
    SELECT m.id, 'r' || lpad(i::text, 2, '0'), i
    FROM m, generate_series(1, 8) AS i
    RETURNING id, position
), c AS (
    INSERT INTO challenges (map_id, name, position, kind)
    SELECT m.id, v.name, v.position, v.kind
    FROM m, (VALUES
        ('t', 1, 'time'), ('dc', 2, 'int'), ('g', 3, 'int'), ('j', 4, 'int'),
        ('f', 5, 'int'), ('fr', 6, 'int'), ('dd', 7, 'int'), ('to', 8, 'int'),
        ('b', 9, 'int'), ('u', 10, 'int'), ('hs', 11, 'int'), ('ds', 12, 'int'),
        ('og', 13, 'int'), ('hg', 14, 'int'), ('dg', 15, 'int'), ('s', 16, 'int'),
        ('is', 17, 'int'), ('fz', 18, 'int'), ('if', 19, 'int'), ('di', 20, 'int'),
        ('h', 21, 'int'), ('1x', 22, 'bool'), ('2x', 23, 'bool'),
        ('3x', 24, 'bool'), ('4x', 25, 'bool')
    ) AS v(name, position, kind)
    RETURNING id, position, kind
)
INSERT INTO challenge_values (room_id, challenge_id, value)
SELECT r.id, c.id,
       CASE c.kind
           WHEN 'time' THEN b.base
           WHEN 'bool' THEN b.base % 2
           ELSE b.base % 30
       END
FROM r, c,
LATERAL (
    SELECT 4000 + r.position * 1300 + c.position * 450
                + (r.position * c.position * 137) % 3500 AS base
) AS b
WHERE (r.position * 7 + c.position) % 11 <> 0;
