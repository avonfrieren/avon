-- Example data: one campaign, one map linked to it, and its challenge list
-- with placeholder best times so the Sum of Best row has something to total.
-- Kept separate from 0004 so the schema migration can be run alone.
WITH camp AS (
    INSERT INTO campaigns (name, is_collab)
    VALUES ('Example Campaign', false)
    RETURNING id
), m AS (
    INSERT INTO maps (name, campaign_id, nb_rooms)
    SELECT 'Example Map', id, 25 FROM camp
    RETURNING id
)
INSERT INTO challenges (map_id, name, position, best_time_ms)
SELECT m.id, c.name, c.position, c.best_time_ms
FROM m,
(VALUES
    ('t',  1,  45230),
    ('dc', 2,  62410),
    ('g',  3,  58990),
    ('j',  4,  71350),
    ('f',  5,  83120),
    ('fr', 6,  95480),
    ('dd', 7,  104760),
    ('to', 8,  88240),
    ('b',  9,  76890),
    ('u',  10, 92310),
    ('hs', 11, 187420),
    ('ds', 12, 66540),
    ('og', 13, 79860),
    ('hg', 14, 112330),
    ('dg', 15, 98770),
    ('s',  16, 54210),
    ('is', 17, 87650),
    ('fz', 18, 73480),
    ('if', 19, 91260),
    ('di', 20, 85940),
    ('h',  21, 121580),
    ('1x', 22, 143210),
    ('2x', 23, 156740),
    ('3x', 24, 168920),
    ('4x', 25, 182350)
) AS c(name, position, best_time_ms);
