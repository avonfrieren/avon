-- Cell values aren't all times — only the 't' challenge is one. The rest
-- count something (deaths, dashes, ...) or are plain pass/fail. Each
-- challenge now declares the kind of value its column holds, and the cell
-- table stores a generic integer:
--   time — milliseconds, formatted/parsed as m:ss.mmm
--   int  — a plain count
--   bool — 0/1, rendered as a checkbox
ALTER TABLE challenges ADD COLUMN kind TEXT NOT NULL DEFAULT 'int'
    CHECK (kind IN ('time', 'int', 'bool'));

ALTER TABLE challenge_times RENAME TO challenge_values;
ALTER TABLE challenge_values RENAME COLUMN time_ms TO value;

-- Adapt the example data: 't' keeps its times, placeholder counts get
-- rescaled down to plausible small numbers, and the x columns are made
-- pass/fail purely to demo the bool kind — change any challenge's kind
-- later with a plain UPDATE like these.
UPDATE challenges SET kind = 'time' WHERE name = 't';
UPDATE challenges SET kind = 'bool' WHERE name IN ('1x', '2x', '3x', '4x');

UPDATE challenge_values cv SET value = cv.value % 30
FROM challenges c WHERE c.id = cv.challenge_id AND c.kind = 'int';

UPDATE challenge_values cv SET value = cv.value % 2
FROM challenges c WHERE c.id = cv.challenge_id AND c.kind = 'bool';
