# Difficulty calculator

Computes a Celeste map's overall difficulty from the difficulty of each
of its rooms. Reached from the sidebar's **diffs** link.

The page currently shows **both** aggregation models side by side (v1 ·
average and v2 · peak+sustain) so they can be compared on the same rooms
while the right model is still being decided.

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

## Comparing the two models — real cases

Same rooms fed to both models (`r = 0.7`, `cap = 2`). Grades shown as
`label · value`.

| Scenario (rooms) | v1 · average | v2 · peak+sustain | What it shows |
| --- | --- | --- | --- |
| 1 GM Green | GM Green · 12.0 | GM Green · 12.0 | A one-room map is that room — both agree. |
| GM Green + 9 Beginner Green | Intermediate Yellow · 3.7 | GM Green · 12.0 | The core split: v1 reads the map's *easy character*, v2 the *hard floor you must clear*. |
| GM Green among 9 Expert Green | Expert Yellow · 9.9 | GM Yellow · 13.2 | v1 dilutes the spike toward the Experts; v2 keeps it near GM. |
| 13-room mix (GM+1 → Expert) | GM Yellow · 13.2 | GM+1 Yellow · 16.2 | The original example. |
| 10 Expert Green (uniform) | Expert Green · 9.0 | Expert Yellow · 10.4 | Uniform map: both ≈ Expert, v2 a touch higher for length. |
| 3 Expert Green | Expert Green · 9.0 | Expert Yellow · 9.9 | Baseline for the pair below. |
| …+ 5 Beginner Yellow | **Advanced Yellow · 6.6** | Expert Yellow · 10.0 | **Adding easy rooms drops v1 a whole tier** (the flaw); v2 is unchanged. |
| 1 GM Green | GM Green · 12.0 | GM Green · 12.0 | Baseline for the pair below. |
| 2 GM Green | GM Green · 12.0 | GM Yellow · 12.6 | v1 ignores the 2nd hard room; v2 rewards the sustain. |

**In short:**

- They **agree** on uniform maps and single rooms.
- They **diverge** when difficulty is uneven:
  - **v1 (average)** rates the map's *typical* difficulty — a mostly-easy
    map reads easy even with a hard spike. Its flaw: adding or removing
    easy rooms shifts the rating (dilution), so *adding easy content
    lowers the difficulty*, which feels wrong.
  - **v2 (peak+sustain)** rates *the floor you must clear, plus how
    sustained it is* — a map is at least as hard as its hardest room,
    more if several hard rooms stack. Its flaw: a single hard spike makes
    the whole map read hard even if 95% of it is trivial.

Neither is "correct" — it's a **choice**: does a hard section *define* a
map's difficulty, or only its *typical* challenge?

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
