# Local iced_wgpu patch

Vendored from crates.io `iced_wgpu` 0.14.0.

Only change: `src/shader/quad.wgsl` draws quad corners as superellipses
(exponent 5) instead of circular arcs, and keeps square quads with fully
rounded corners as circles. It treats `border.radius` as the app's circular-
style radius and expands it to a `2.25x` extent, matching
`nade_constants::squircle::EXTENT`.

Apply the same change when bumping `iced_wgpu`.
