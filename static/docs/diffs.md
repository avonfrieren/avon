# Difficulty calculator

Computes a Celeste map's overall difficulty from the difficulty of each
of its rooms. Reached from the sidebar's **diffs** link.

The page currently shows **four** aggregation models side by side (v1 ·
average, vI · ratio², vI2 · abs gap, v2 · peak+sustain) so they can be
compared on the same rooms while the right model is still being decided.

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
several rooms at *nearly* the peak difficulty. So vI stays
between the peak and v2 — `peak ≤ vI ≤ v2` always, while v1 can dip
*below* the peak (dilution). vI is a middle ground: monotone and
peak-anchored like v2, but crediting only genuinely-near-peak difficulty.

## vI2 — the absolute-gap alternative

vI and v2 measure a room's closeness to the peak as a **ratio**
(`dᵢ/peak`). vI2 measures it as the **absolute gap** `dᵢ − peak` (in
encoded units), decayed exponentially: `gap_factor = (1.3^(dᵢ − peak))²`.
It keeps v2's rank weighting (`r^i`).

The consequence is **scale-independence**: with vI2, a room *N encoded
units below the peak* always contributes the same, regardless of how hard
the peak is (each unit ≈ ×0.59, a full tier ≈ ×0.21). The ratio models
don't: a room one tier under a GM+5 peak (ratio ≈ 0.9) counts far more
than one tier under an Expert peak (ratio ≈ 0.67).

Concretely, feeding rooms at the *same tier-distances* below very
different peaks:

- **GM+5 peak, rooms 1–2 tiers below** → vI **+0.74**, vI2 **+0.26** above peak
- **Expert peak, same tier-distances** → vI **+0.38**, vI2 **+0.26** above peak

vI2's bonus is identical in both (absolute gaps are identical); vI's grows
with the peak. The flip side: for rooms *exactly at* the peak, vI2 keeps
v2's full weighting (`r^i`) where vI discounts it (`r^(i+1)`), so vI2 ≈ v2
on uniform/stacked-peak maps while vI sits lower. vI2 also stays in
`peak ≤ vI2 ≤ v2`.

In one line: **vI discounts by *relative* distance, vI2 by *absolute*
tier-distance.**

## The value vs the label

D is a continuous number (e.g. 9.92), always shown exactly. The **label**
(e.g. "Expert Yellow") is that value mapped to the nearest discrete grade
— this rounding is display-only and never affects the computed value.

## Comparing the models — real cases

Same rooms fed to all four (`r = 0.7`, `cap = 2`). Values are the raw
numbers (Expert Green = 9, GM Green = 12, GM+1 Green = 15, GM+5 Green =
27; see the encoding above).

| Scenario (rooms) | v1 | vI | vI2 | v2 | What it shows |
| --- | --- | --- | --- | --- | --- |
| GM Green + 9 Beginner | **3.7** | 12.0 | 12.0 | 12.0 | v1 dilutes to the easy rooms; the others hold the GM floor. |
| GM Green among 9 Expert | 9.9 | 12.7 | 12.4 | 13.2 | All three sustains stay near the GM peak; vI2 lowest here. |
| 13-room mix (GM+1 → Expert) | 13.2 | 15.9 | 15.6 | 16.2 | The original example. |
| …+ 5 Beginner Yellow to 3 Expert | **6.6** | 9.7 | 9.7 | 10.0 | **Adding easy rooms drops v1 a tier**; the sustains ignore them. |
| 10 Expert (uniform) | 9.0 | 10.1 | **10.4** | 10.4 | Peak-equal rooms: **vI2 = v2** (full weight), vI lower (rank penalty). |
| 2 GM Green | 12.0 | 12.4 | **12.6** | 12.6 | Same — vI2 tracks v2 on stacked-peak rooms, vI trails. |
| **GM+5 peak, rooms 1–2 tiers below** | 24.5 | 27.7 | 27.3 | 28.1 | Bonus above peak (27): vI **+0.7**, **vI2 +0.3**, v2 +1.1. |
| **Expert peak, same tier-distances** | 6.5 | 9.4 | 9.3 | 9.8 | Bonus above peak (9): vI **+0.4** (shrank!), **vI2 +0.3** (same), v2 +0.8. |

The last two rows are the heart of **vI vs vI2**: identical tier-distances
below the peak, but very different peaks. **vI2 gives the same bonus
(+0.3) both times** — it only cares about absolute tier-distance — while
**vI's bonus shrinks with a lower peak** (+0.7 → +0.4), because its ratio
metric makes near-peak rooms count more when the peak is high.

**In short:**

- All four **agree** on uniform maps and single rooms.
- They **diverge** when difficulty is uneven:
  - **v1 (average)** rates the map's *typical* difficulty — a mostly-easy
    map reads easy even with a hard spike. Its flaw: adding or removing
    easy rooms shifts the rating (dilution), so *adding easy content
    lowers the difficulty*, which feels wrong.
  - **v2 (peak+sustain)** rates *the floor you must clear, plus how
    sustained it is* — at least as hard as the hardest room, more if
    several hard rooms stack. Its flaw: one hard spike makes the whole
    map read hard even if 95% is trivial.
  - **vI (ratio²)** and **vI2 (abs gap)** are both middle grounds between
    the peak and v2, differing only in how a room's closeness to the peak
    is measured: vI by *relative* ratio (near-peak rooms count more when
    the peak is high), vI2 by *absolute* tier-distance (scale-independent,
    and it keeps full weight for peak-equal rooms where vI discounts them).

Neither is "correct" — it's a **choice**: does a hard section *define* a
map's difficulty, or only its *typical* challenge, and is "close to the
peak" *relative* or *absolute*?

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
