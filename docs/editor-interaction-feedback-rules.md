# Editor Interaction Feedback Rules

This document defines the visual feedback contract for hover, focus, preview, and temporary drag layers in the Chemcore editor. These rules keep interaction feedback consistent across small and large documents.

## Visual Handles

- Ordinary object control handles use a hollow circular handle with a 1.5 CSS px radius.
- Endpoint hover handles use the same 1.5 CSS px visual radius when they are visible.
- Endpoint hit testing remains independent from visual size. The endpoint hit radius is 10 CSS px.
- Selection resize handles and arrow endpoint style handles are separate interaction systems and keep their own sizing rules.

## Endpoint Feedback

Endpoint hover is chemical editing feedback, not generic object creation feedback.

- The bond tool may show endpoint hover while drawing or extending bonds.
- The bond tool may show the preview end handle while dragging a bond.
- Non-bond object creation tools must not show endpoint hover circles or preview end dots.
- Non-bond object creation tools may still use endpoints internally as placement anchors, but that anchoring must not create endpoint hover visuals.
- Text and delete interactions may keep their own endpoint-specific feedback because their command targets are endpoints.

## Temporary Layers

The editor has more than one transient visual layer:

- the engine interaction render list,
- the editor overlay layer,
- the canvas drag preview layer,
- document preview transforms and masks.

Any completed, canceled, or abandoned pointer interaction must clear every transient layer that it could have touched. A stale animation frame or async pointer move must not be allowed to repaint an old hover or preview after the interaction has committed.

## Regression Expectations

Tests that cover object creation and large-document editing should assert:

- ordinary object handles and endpoint hover handles use the configured visual radius,
- non-bond object tools do not show endpoint hover visuals while hovering atoms,
- pointer-up after object creation clears hover and preview roles from all transient layers,
- clearing temporary feedback does not require a full document render list refresh.
