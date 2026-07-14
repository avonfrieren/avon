# Difficulty calculator

Computes a Celeste map's overall difficulty from the difficulty of each
of its rooms. Reached from the sidebar's **diffs** link.

The page shows **three** aggregation models side by side (v1 · average,
v2 · peak+sustain, v3 · Ildyia) so they can be compared on the same rooms
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

## v3 — Ildyia's refinement of v2

**v3 was designed by Ildyia**, and it improves v2 with a single, elegant
change — the kind that looks obvious only once someone has seen it.

v2 measures how close a room is to the peak as a **ratio**, `dᵢ/peak`.
Ildyia's observation: on a *tier* scale, that's the wrong yardstick. The
ratio depends on how hard the peak happens to be — a room one tier under a
GM+5 peak scores `24/27 ≈ 0.9` and counts heavily, while the *same
one-tier gap* under an Expert peak scores `6/9 ≈ 0.67` and counts far
less. The model's sense of "near the peak" drifts with the peak's height.

Her fix keeps everything else about v2 (same `D = peak + cap·(1 − rᴱ)`
shape, same rank weighting `r^i`) and swaps only the gap factor for an
**absolute** one — an exponential decay of the raw tier-distance:

`gap_factor = (1.3^(dᵢ − peak))²`

Now a room *N encoded units below the peak* contributes the same amount
whatever the peak is (each unit ≈ ×0.59, a full tier ≈ ×0.21). "Near the
peak" finally means the same thing at Expert and at GM+5.

Why it's clever:

- **Scale-independent** — the property v2 lacked. Same tier-distances give
  the same sustained bonus regardless of the peak's magnitude.
- **Minimal** — one term changes; every good property of v2 survives
  (monotone, bounded, `peak ≤ v3 ≤ v2`, easy rooms → weight ~0).
- **Well-tuned** — base 1.3 keeps real weight on rooms a shade or two below
  the peak while annihilating far-below ones (an Expert under a GM peak),
  which matches how a sustained-difficulty map actually feels.

Concretely, the same tier-distances below very different peaks:

- **GM+5 peak, rooms 1–2 tiers below** → v3 **+0.26** above peak
- **Expert peak, same tier-distances** → v3 **+0.26** above peak

Identical — exactly the point. (v2, by ratio, gives **+1.07** then
**+0.79** for the same two — its bonus drifts with the peak.)

## The value vs the label

D is a continuous number (e.g. 9.92), always shown exactly. The **label**
(e.g. "Expert Yellow") is that value mapped to the nearest discrete grade
— this rounding is display-only and never affects the computed value.

## Comparing the models — real cases

Same rooms fed to all three (`r = 0.7`, `cap = 2`). Values are the raw
numbers (Expert Green = 9, GM Green = 12, GM+1 Green = 15, GM+5 Green =
27; see the encoding above).

| Scenario (rooms) | v1 | v2 | v3 | What it shows |
| --- | --- | --- | --- | --- |
| GM Green + 9 Beginner | **3.7** | 12.0 | 12.0 | v1 dilutes to the easy rooms; v2 & v3 hold the GM floor. |
| GM Green among 9 Expert | 9.9 | 13.2 | 12.4 | Both sustains stay near the GM peak; v3 a touch tighter. |
| 13-room mix (GM+1 → Expert) | 13.2 | 16.2 | 15.6 | The original example. |
| …+ 5 Beginner Yellow to 3 Expert | **6.6** | 10.0 | 9.7 | **Adding easy rooms drops v1 a tier**; the sustains ignore them. |
| 10 Expert (uniform) | 9.0 | 10.4 | 10.4 | Peak-equal rooms: **v3 = v2** (v3 keeps v2's rank weighting). |
| **GM+5 peak, rooms 1–2 tiers below** | 24.5 | 28.1 | 27.3 | Bonus above peak (27): v2 **+1.1**, **v3 +0.3**. |
| **Expert peak, same tier-distances** | 6.5 | 9.8 | 9.3 | Bonus above peak (9): v2 **+0.8** (shrank), **v3 +0.3** (same). |

The last two rows are the heart of **v2 vs v3**: identical tier-distances
below the peak, but very different peaks. **v3 gives the same bonus (+0.3)
both times** — it only cares about absolute tier-distance — while **v2's
bonus drifts with the peak** (+1.1 → +0.8), because its ratio metric makes
near-peak rooms count more when the peak is high.

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
  - **v3 (Ildyia)** is v2 made scale-independent: same peak-anchored,
    monotone, bounded shape, but "near the peak" is an *absolute*
    tier-distance instead of a ratio, so the same gap counts the same at
    every difficulty. Between the peak and v2 (`peak ≤ v3 ≤ v2`).

Neither is "correct" — it's a **choice**: does a hard section *define* a
map's difficulty, or only its *typical* challenge, and is "close to the
peak" *relative* or *absolute*?

## Changelog

Each entry is the calculator version (`major.minor`) in which the
behavior was introduced or changed.

### 3.0 — by Ildyia

- **v3, designed by Ildyia**: refines v2 by measuring a room's closeness
  to the peak as an *absolute* tier-distance
  (`gap_factor = (1.3^(dᵢ − peak))²`) instead of a ratio. Makes the
  sustained bonus **scale-independent** — the same tier-gap counts the
  same at every difficulty — while keeping all of v2's properties
  (monotone, bounded, `peak ≤ v3 ≤ v2`). An elegant one-term change.

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
