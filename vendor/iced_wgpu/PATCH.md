# Local iced_wgpu patch

Vendored from crates.io `iced_wgpu` 0.14.0.

Only change: `src/shader/quad.wgsl` draws quad corners as superellipses
(exponent 5) instead of circular arcs. `border.radius` is the corner extent,
matching `iced-squircle` 1.0 and the app's design tokens. Square quads with a
radius of exactly half a side stay circles.

Apply the same change when bumping `iced_wgpu`.
