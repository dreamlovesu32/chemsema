# ChemCore Bond Rendering Rules

This document records the bond rendering and bond-contact rules currently used by the Rust rendering kernel. It fixes geometry definitions, implementation boundaries, and refactor baselines.

In this document, "should", "must", and "must not" are normative requirements. "May" and "allowed" describe implementation flexibility.

## Goals

- Bond rendering should be defined uniformly by the Rust kernel and should not rely on secondary viewer patches.
- Similar bond types should share geometry models as much as possible, reducing local heuristics split by bond type.
- Node contact should be determined first by contours and endpoint profiles, avoiding accidental shapes introduced by SVG `linecap` or foreground styles.
- Preview state and committed state should reuse the same geometry.

## Scope

- This document constrains only the fragment path.
- The `legacy molblock` path only needs compatible output; it does not need to satisfy each new geometry rule here.
- This document describes 2D geometry output. Foreground interaction state, hover decoration, and selection-box styling are constrained by interaction-layer documents.

## Terms

- Main bond: the principal-axis body that determines bond contact and node occupancy.
- Secondary line: the offset subsidiary line in one-sided double bonds and triple bonds.
- Body: the complete polygon before knockout splits it into segments.
- Endpoint profile: the contour chain actually consumed by one bond at one endpoint; it closes the final polygon.
- Keep as-is: keep the standard body outline of the bond and do not alter the cap contour for contact.
- Retreat: do not change outline topology; only shorten or move backward along the axis so the bond does not press into the contact object.
- Same side: relative to the node local normal, the secondary line or outer line is on the same side as the current reference line.
- Straight-through: two axes are approximately `180°` opposite and collinear.

## Overall Classification

Current bonds are divided into three classes:

1. Objects with a main bond.
2. Objects without a main bond but with a fixed body shape.
3. Legacy molblock path.

This document discusses only the first two classes in the fragment path.

### Objects With A Main Bond

- ordinary single bond
- bold solid bond
- ordinary dashed bond
- hash bond
- one-sided double bond
- centered double bond
- triple bond
- solid wedge bond
- hashed wedge bond

### Unified Geometry Vocabulary

- All solid bonds are treated as polygons.
- Ordinary single bonds and bold solid bonds are geometrically the same except for width.
- Solid wedge bonds are treated as trapezoids. The narrow end is a very short edge.
- Ordinary dashed bonds are "body polygon + white knockout segments".
- Hash bonds are "bold solid body rectangle + denser white knockout segments".
- Hashed wedge bonds are "solid wedge body trapezoid + white knockout segments".

## ChemDraw Drawing Template Parameters

Default and ACS Document 1996 use two independent sets of template parameters. When switching templates, existing bonds and future new bonds should both switch to the corresponding template parameters.

Key differences currently aligned with ChemDraw:

| Parameter | Default | ACS Document 1996 | Notes |
| --- | ---: | ---: | --- |
| Bond length | `30.0pt` | `14.4pt` | Existing structure geometry is scaled by this ratio when switching templates |
| Ordinary line width | `1.0pt` | `0.6pt` | Used for ordinary bonds, narrow wedge ends, and shape line width |
| Bold bond width | `4.0pt` | `2.0pt` | Template width for bold solid bonds |
| Solid/hollow wedge wide end | `6.0pt` | `3.0pt` | `1.5 * BoldWidth`; hollow wedges share the solid wedge outline |
| Label clip natural outset | source `MarginWidth` | source `MarginWidth` | Absolute pt value from the source template; it does not scale with label font size |
| Label clip circle radius | `2 * MarginWidth` | `2 * MarginWidth` | Absolute pt value derived from the source template margin |
| Hash spacing | `2.7pt` | `2.5pt` | Template spacing for the hash family |
| Bond spacing | `12%` | `18%` | Double-bond spacing percentage used with the actual bond length and the line-width floor |
| Margin width | `2.0pt` | `1.6pt` | White margin around upper bonds at non-endpoint bond-bond crossings |

Wedge wide-end width, label clip natural outset, label clip circle radius, and margin width are template parameters and should not be inferred backward from bond length. CDXML import derives the solid/hollow wedge wide end as `1.5 * BoldWidth` from the source template and uses the source label clipping numbers directly; do not infer an "ACS" label clipping mode from `MarginWidth`.

For CDXML imports, label clipping geometry is parameterized by the source `MarginWidth`: natural outset is exactly `MarginWidth`, and circular anchor radius is exactly `2 * MarginWidth`. These values are absolute points and must not be multiplied by the label font size or glyph height.

## Unified Decision Order

Each bond endpoint should compute final geometry in this order:

1. Determine the standard body shape of the bond.
2. If the node has a label, perform label clipping first.
3. Decide whether this endpoint enters node-contact rules.
4. If it enters contact, choose by bond type:
   - main-bond intersection
   - secondary-line intersection
   - retreat only
   - keep as-is
5. Apply dash/hash/knockout to the final body.
6. Preview state and committed state must use the same flow.

No implementation may treat viewer stroke, line join, or line cap as part of the geometry definition.

## Main-Bond Contact

"Main-bond contact" means the contact region is determined by the main-bond contour.

### Two-Bond Contact

This applies to objects "with a main bond": ordinary single bonds, bold solid bonds, ordinary dashed bonds, hash bonds, one-sided double-bond main lines, triple-bond main lines, solid wedge axes, hashed wedge axes, and similar cases.

Rules:

1. The two bonds share a center point at the node.
2. Each bond has two main-bond contour lines at that endpoint.
3. Compute the `inner-inner` intersection and the `outer-outer` intersection between the two bonds.
4. These two intersections form the contact-region boundary.
5. Each bond consumes its own endpoint profile and directly generates its own polygon, without extra mask patches.

Additional rules:

- Intersections may use extended lines; the intersection does not need to lie within the original segment length.
- For very small angles, a miter limit may be used.
- If a miter limit truncates the intersection, each bond must still own clipping points that lie on its own extended contour lines, and must not fall back to arbitrary empirical midpoints.

### Three-Or-More-Bond Contact

Rules:

1. Sort incident bonds around the node.
2. Process only adjacent pairs, not all combinations.
3. Intersect the inner contour lines of every adjacent pair to form a ring of points around the node.
4. Each bond consumes its own endpoint contour chain.
5. In a triple-bond node, a solid bond may naturally become a pentagon; with more bonds, the vertex count continues to grow.

Additional rules:

- Multi-bond nodes process only adjacent pairs, not all combinations.
- Intersections from non-adjacent pairs must not be used for center-node topology.
- If a bond class is defined as "keep as-is", that bond does not participate in center-node occupancy; other bonds retreat.

### Straight-Through Exception

- `180°` straight-through cases do not require intersections.
- Centered double bonds do not extend when the included angle is greater than `162°`.

## Single Bonds And Bold Solid Bonds

- Ordinary single bonds and bold solid bonds both render as quadrilaterals.
- They differ only by width parameters.
- In contact, both are main-bond contact objects.

## Ordinary Dashed Bonds

- The body of an ordinary dashed bond is still a standard rectangular main bond.
- White cut segments are placed at equal intervals along the main axis.
- Contact uses the body rectangle.
- Black segment length may vary according to dash rules; ordinary dashed bonds do not require strictly equal black segments like the hash family.

## Hash Bonds

Hash bonds are a separate model: a bold solid body plus white cut segments.

- The body is always a bold solid rectangle.
- Black segment lengths must be equal.
- White segment lengths may vary within a range.
- The number of white segments changes only when the total length exceeds the range allowed by the current segment count.
- Black segment length has higher priority than equal spacing of white segments.

### Hash Bonds And Ordinary Main Bonds

- Keep the standard rectangle and do not actively deform it to meet other bonds.
- In a multi-bond node, other bonds retreat and the hash bond stays as-is.

### Hash Bonds And Centered Double Bonds

- Do not intersect and do not change the rectangular cap.
- Retreat only along the hash bond's own center line.
- The retreat reference is the correct-side outer line of the centered double bond.
- White cut segments follow the actual retreated rectangle, not the old length.

## Solid Wedge Bonds

- The body is a trapezoid.
- The wide-end width is fixed, and the narrow-end width is fixed as a very short top edge.
- In ordinary main-bond contact, the wide end may deform by contour intersection.
- If the wide end meets a label, solid wedge label clipping uses the centerline-clipped endpoint only. Do not add an extra wide-end margin retreat for solid wedges.
- When length changes, endpoint width definitions should be preserved first, then trapezoid side edges are updated.

## Hashed Wedge Bonds

- The body is a standard trapezoid.
- Black segments are arranged along the trapezoid main axis.
- Black segment lengths remain equal.
- White segment lengths may vary within range; the count changes only when out of range.
- If either endpoint meets a label, label clipping is identical to solid wedge label clipping: use the centerline-clipped endpoint and do not add an extra wide-cap or hash-family retreat.

### Hashed Wedge Bonds And Ordinary Main Bonds

- Keep the standard trapezoid and do not actively intersect to reshape the wide end.
- In multi-bond nodes, prefer preserving its own body shape and let other bonds retreat.

### Hashed Wedge Bonds And Centered Double Bonds

- Do not intersect at the wide end.
- Retreat only along the hashed wedge's own axis.
- The wide-end width stays fixed, so the trapezoid naturally deforms after length changes.
- White cut segments must follow the actual retreated trapezoid.

## One-Sided Double Bonds

A one-sided double bond consists of one main bond and one secondary line. For CDXML-compatible spacing, `bondSpacing` is a percentage applied to the actual bond length, but ChemDraw also enforces a line-width floor. The inner gap is:

```text
max(actual bond length * bondSpacing / 100 - lineWidth, 1.5 * lineWidth)
```

The center distance is that inner gap plus half of each rendered line width. For two normal-weight lines, this simplifies to:

```text
max(actual bond length * bondSpacing / 100, 2.5 * lineWidth)
```

This is not a style-template branch: ACS, default, and custom CDXML documents use the same rule with their source `LineWidth`, `BondLength`, and `BondSpacing` values.

### Automatic Side Selection

When CDXML or an editing operation gives only `Order="2"` and no explicit `DoublePosition`, the engine should decide automatically using ChemDraw-compatible rules:

1. If either end of the bond directly connects to another double bond, use centered double.
2. If neither end has other connected bonds, use centered double.
3. If one terminal end has substituent bonds on both left and right sides while the other end has no substituent bond, use centered double.
4. If the bond belongs to a ring, first choose the reference ring using ring-selection rules, then place the secondary line inside that reference ring:
   - a fully alternating six-membered ring has priority over shorter fused small rings
   - then compare complete alternation, alternation match count, closeness to six-membered rings, and path length in order
5. For non-ring structures, compute the signed projection sum of all connected substituent bonds along the left normal of the begin-to-end axis:
   - positive sum: place on the left side
   - negative sum: place on the right side
   - exact tie: default to the right side

### Main Bond

- The main bond is an ordinary main-bond contact object.
- It contacts single bonds, bold solid bonds, triple bonds, wedges, and similar objects by the main-bond contact rules.

### Secondary Line

- The outer secondary line keeps its original shape by default and does not participate in ordinary contact.
- Only the inner secondary line participates in contact intersections.
- At terminal ends, it is the same length as the main bond.
- At non-terminal ends, it is shortened by "main-bond spacing * `sqrt(3) / 3`".
- When the angle between the main bond and the connected main bond is less than `90°`, the secondary line may retreat; however, if the other bond has a same-side secondary line, same-side secondary-line intersection still has priority.

### Acute-Angle Retreat

- If the main bond connects to a main bond and that side's angle is less than `90°`, the secondary line may retreat.
- If the other bond also has a same-side secondary line, same-side secondary lines still intersect directly.
- If the other secondary line is not on the same side, use retreat.
- If it meets a centered double bond, use the centered double's center line as the retreat reference.

## Centered Double Bonds

A centered double bond consists of two independent sub-lines.

Rules:

- Each sub-line evaluates its own endpoints independently.
- Each end and each side independently decides whether to extend to another bond's contour.
- Near-straight-through cases greater than `162°` do not extend.
- `180°` straight-through keeps the original shape.
- When contacting the hash family, the centered double bond is the reference object; it does not reverse-modify hash-family outlines.
- Applies to:
  - solid-solid centered double bonds
  - dashed-solid centered double bonds
  - dashed-dashed centered double bonds

## Triple Bonds

- The triple-bond main line is a main-bond contact object.
- The two outer lines independently evaluate their endpoint profiles.
- At terminal ends, outer lines remain full length.
- Similar to one-sided double-bond secondary lines, outer-line contact is determined by each line's own contour.

## Hash-Family Priority In Multi-Bond Nodes

If one node connects to multiple bonds and one of them is a `hash bond` or `hashed wedge`:

- `hash bond` / `hashed wedge` stays as-is.
- Other bonds retreat.
- A small white gap is intentionally left; complete seamless contact is not required.

## Decision Priority

If multiple rules hit the same endpoint, apply them in this order:

1. label clipping
2. hash-family keep-as-is rule
3. centered double near-straight-through no-extension rule
4. main-bond contact intersection
5. one-sided double secondary-line same-side intersection
6. one-sided double secondary-line acute-angle retreat
7. terminal equal-length / endpoint fixed-length rules
8. dash / hash knockout cuts

Later rules must not overturn geometry topology already determined by earlier rules; they may only continue tightening within remaining degrees of freedom.

## Labels And Clipping

- If a node has a visible label, perform label clipping first.
- Label clipping occurs before bond contact.
- Final bond geometry should satisfy both label clipping and node-contact rules.
- Endpoint label clipping is defined only by the glyph clip polygons generated
  from glyph geometry and source `MarginWidth`. Do not add a separate
  per-bond `labelClipMargin` or post-hoc retreat value.
- `MarginWidth` natural extension applies to every visible glyph edge,
  including internal stroke bays and stroke ends that remain inside the
  glyph's overall bounding box. Runtime mapping must not scale only the
  outside of the whole glyph bbox.
- For rendered bonds with thickness, the whole bond body must be clipped out of
  the glyph clip polygon. Ordinary, dashed, bold, and multi-line bonds should
  evaluate the center line plus both body boundary lines for the current visual
  half-width and use the largest retreat required by those glyph intersections.

## Bond-Bond Crossings And White Margins

Non-endpoint bond-bond crossings are intersections of two internal bond segments that do not share begin/end nodes. This rule covers bond-vs-bond only, not bond-vs-text, bond-vs-shape, or bond-vs-arrow.

Rules:

- Within the same molecule fragment, the later-rendered bond is considered the upper bond.
- Before drawing the upper bond, generate a white knockout around it using that bond's `marginWidth`, so lower bonds break at the crossing.
- White margin width uses the upper bond's template parameter: Default is `2.0pt`, ACS Document 1996 is `1.6pt`.
- Knockout direction follows the upper bond axis; width covers the upper bond visible width plus both side margins, and length covers the lower bond visible width with crossing-angle compensation.
- Bonds sharing endpoints still use the node contact kernel, not this white-margin rule.
- Nearly parallel or overlapping bonds do not enter the first version of the bond-bond crossing white-margin rule.

## Preview

- Dragging to draw a bond should not draw a fake overlay bond.
- Preview state calls the same `render_document` through a temporary document.
- Therefore preview and committed output must share the same contact geometry.

## Refactor Constraints

When splitting the `render` module later, keep these constraints:

- Do not change the geometry rules in this document.
- Prefer splitting high-level dispatch from low-level geometry; do not continue changing rules in the same refactor.
- After every split, pass:
  - `cargo test`
  - `npm run build:engine-wasm`

## Test Baseline

At least one layer of automated tests must cover these rule classes:

- main-bond contact side selection and straight-through decisions
- centered double `162°` threshold
- intersection fallback after miter-limit truncation
- hash-family rhythm allocation: equal black segments and floating white segments
- boundary pairing must not choose trivial assignments incorrectly
- hash family stays as-is in multi-bond nodes, while other bonds retreat

Suggested test layers:

- Low-level unit tests: directly test intersection, side selection, and rhythm-allocation helpers.
- Render-level tests: test output primitive counts, types, and key vertex relationships.
- Foreground validation: only confirms that the viewer has no stroke / antialiasing artifacts; it does not define geometry.

## Scope Boundaries

- SVG antialiasing details are handled by the concrete rendering backend.
- Pixel-level differences across browsers are handled by visual regression tolerance.
- Debug panels, debug SVGs, and development patches are development tools.

## Known Responsibility Boundaries Suitable For Splitting

- `render_document` and object-level dispatch
- high-level bond render dispatch
- main bond contact kernel
- bond geometry / intersection helpers
- render primitive definitions and push helpers
- legacy mol render compatibility layer
