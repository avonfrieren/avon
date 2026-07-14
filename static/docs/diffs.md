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

The room values are sorted hardest-first, then averaged with a weight
that decays geometrically by rank:

`D = Σ (rⁱ · d_sorted_i) / Σ rⁱ`, i from 0, with `r ∈ ]0,1[`
(default 0.7).

Properties:

- Easy rooms land at the tail with weight ~0 — no dilution, and removing
  easy rooms barely changes D.
- A lone hard room is pulled down by its softer neighbours (one GM among
  Experts ⇒ map ≈ Expert, not GM).
- Several hard rooms grouped push D toward the peak ("sustained
  difficulty").

`r` is the only knob: → 0 = max (the single hardest room), → 1 = plain
average, sweet spot 0.6–0.75, to calibrate on consensus maps.

## The value vs the label

D is a continuous number (e.g. 9.92), always shown exactly. The **label**
(e.g. "Expert Yellow") is that value mapped to the nearest discrete grade
— this rounding is display-only and never affects the computed value.

## Changelog

Each entry is the calculator version (`major.minor`) in which the
behavior was introduced or changed.

### 1.0

- Initial calculator: tier×3+shade encoding, geometric rank-weighted
  average (`r` default 0.7), round-to-nearest labeling.
