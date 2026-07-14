# Difficulty calculator

Computes a Celeste map's overall difficulty from the difficulty of each
of its rooms. Reached from the sidebar's **diffs** link.

The page currently shows **three** aggregation models side by side (v1 ·
average, vI · strict sustain, v2 · peak+sustain) so they can be compared
on the same rooms while the right model is still being decided.

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

## vI — the strict-sustain alternative

A community-proposed variant of v2 (same `D = peak + cap·(1 − rᴱ)` shape),
differing only in how the effective count `E` is built:

- each non-peak room is scaled by `(dᵢ/peak)²` instead of `dᵢ/peak` —
  **squared**, so rooms that aren't *very* close to the peak are heavily
  discounted;
- its rank weight carries one extra factor of r (`r^(i+1)` vs `r^i`).

Both make vI stricter than v2: a map only climbs above its peak if it has
several rooms at *nearly* the peak difficulty. So the three models order
cleanly — `peak ≤ vI ≤ v2` always, while v1 can dip *below* the peak
(dilution). vI is the middle ground: monotone and peak-anchored like v2,
but crediting only genuinely-near-peak difficulty.

## The value vs the label

D is a continuous number (e.g. 9.92), always shown exactly. The **label**
(e.g. "Expert Yellow") is that value mapped to the nearest discrete grade
— this rounding is display-only and never affects the computed value.

## Comparing the models — real cases

Same rooms fed to all three (`r = 0.7`, `cap = 2`). Values are the raw
numbers (Expert Green = 9, GM Green = 12, GM+1 Green = 15; see the
encoding above).

| Scenario (rooms) | v1 | vI | v2 | What it shows |
| --- | --- | --- | --- | --- |
| 1 GM Green | 12.0 | 12.0 | 12.0 | One room = its own grade; all agree. |
| GM Green + 9 Beginner Green | **3.7** | 12.0 | 12.0 | v1 dilutes toward the easy rooms; vI & v2 hold the GM floor (easy rooms count ~0). |
| GM Green among 9 Expert Green | 9.9 | 12.7 | 13.2 | v1 → Expert; vI barely leaves the peak (Experts squared down); v2 credits them more. |
| 13-room mix (GM+1 → Expert) | 13.2 | 15.9 | 16.2 | The original example — vI sits just under v2. |
| 10 Expert Green (uniform) | 9.0 | 10.1 | 10.4 | Uniform: all ≈ Expert, small sustain. |
| 3 Expert Green | 9.0 | 9.7 | 9.9 | Baseline for the row below. |
| …+ 5 Beginner Yellow | **6.6** | 9.7 | 10.0 | **Adding easy rooms drops v1 a whole tier** (the flaw); vI & v2 unchanged. |
| 2 GM Green (vs 1 above) | 12.0 | 12.4 | 12.6 | v1 ignores the 2nd hard room; vI a little, v2 more. |
| 5 GM+1 Green (sustained peak) | 15.0 | 15.9 | 16.2 | Fully sustained: vI ≈ v2, both above the peak. |

**In short:**

- All three **agree** on uniform maps and single rooms.
- They **diverge** when difficulty is uneven:
  - **v1 (average)** rates the map's *typical* difficulty — a mostly-easy
    map reads easy even with a hard spike. Its flaw: adding or removing
    easy rooms shifts the rating (dilution), so *adding easy content
    lowers the difficulty*, which feels wrong.
  - **v2 (peak+sustain)** rates *the floor you must clear, plus how
    sustained it is* — at least as hard as the hardest room, more if
    several hard rooms stack. Its flaw: one hard spike makes the whole
    map read hard even if 95% is trivial.
  - **vI (strict sustain)** is the middle ground: like v2 but only rooms
    *genuinely* near the peak add to the bonus, so it hugs the peak
    unless the hard difficulty is tightly clustered.

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
