# Difficulty calculator

Computes a Celeste map's overall difficulty from the difficulty of each
of its rooms. Reached from the sidebar's **diffs** link.

## Grading a room

Each room is graded on an open-ended **tier** scale, each tier split into
three **shades** (Green < Yellow < Red):

Beginner, Intermediate, Advanced, Expert, GM, GM+1, GM+2, … (unbounded
upward).

A grade encodes to a number: `value = tier_base × 3 + shade`, with
Green = 0, Yellow = 1, Red = 2 and Beginner = 0, Intermediate = 1,
Advanced = 2, Expert = 3, GM = 4, GM+1 = 5, GM+n = 4 + n. So Expert
Green = 9, GM Green = 12, GM+1 Green = 15.

## Map difficulty

The map is anchored at its **peak** (its hardest room), then raised by a
bounded **sustained-difficulty** bonus:

`D = peak + cap · (1 − rᴱ)`

where `E` is an "effective count" of the near-peak rooms: the rooms
sorted hardest-first (beyond the peak) each add `rⁱ · (dᵢ / peak)` to E,
i.e. a rank-weighted amount scaled by how close the room is to the peak.

Properties:

- **Monotone**: adding a room never lowers D (E only grows). A map is
  always at least as hard as its hardest room.
- **Bounded**: D never exceeds `peak + cap`, so it stays on the tier
  scale — length alone can't inflate an easy map.
- **Sustained difficulty**: a run of near-peak rooms pushes E up and D
  toward the `peak + cap` ceiling; a lone hard room among easy ones stays
  near the peak.
- Easy rooms fall to the tail with weight ~0 — they contribute nothing,
  and removing them barely changes D.

Two knobs, to calibrate on consensus maps:

- `r ∈ ]0,1[` (default 0.7) — how fast a room's rank weight decays.
- `cap` (default 2.0) — the most the sustained bonus can add above the
  peak.

## The value vs the label

D is a continuous number (e.g. 9.92), always shown exactly. The **label**
(e.g. "Expert Yellow") is that value mapped to the nearest discrete grade
— this rounding is display-only and never affects the computed value.

## Changelog

Each entry is the calculator version (`major.minor`) in which the
behavior was introduced or changed.

### 2.0

- Aggregation changed from a rank-weighted **average** to a
  **peak-anchored** model with a bounded sustained bonus
  (`D = peak + cap·(1 − rᴱ)`). Fixes the average's flaw where adding
  rooms could *lower* a map's difficulty; the result is now monotone and
  a map is always at least as hard as its hardest room. Adds the `cap`
  knob (default 2.0).

### 1.0

- Initial calculator: tier×3+shade encoding, geometric rank-weighted
  average (`r` default 0.7), round-to-nearest labeling.
