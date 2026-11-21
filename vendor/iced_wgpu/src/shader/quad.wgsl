struct Globals {
    transform: mat4x4<f32>,
    scale: f32,
}

@group(0) @binding(0) var<uniform> globals: Globals;

/// Signed distance to a superellipse corner of the given extent.
///
/// `q` is the corner offset measured from the inner (straight-edge) rectangle.
/// The corner is an L^n ball instead of an L^2 (circular) one. Dividing by the
/// magnitude of the implicit gradient keeps the result a usable distance, so
/// borders and antialiasing stay one pixel wide around the curve.
fn superellipse_corner(q: vec2<f32>, extent: f32) -> f32 {
    let n: f32 = 5.0;
    let m = max(q, vec2<f32>(0.0));
    let x2 = m.x * m.x;
    let y2 = m.y * m.y;
    let x4 = x2 * x2;
    let y4 = y2 * y2;
    let r = x4 * m.x + y4 * m.y;

    if (r <= 1e-12) {
        return -extent;
    }

    let d = pow(r, 1.0 / n);
    let gradient = max((d / r) * sqrt(x4 * x4 + y4 * y4), 1e-6);
    return (d - extent) / gradient;
}

fn rounded_box_sdf(p: vec2<f32>, size: vec2<f32>, corners: vec4<f32>) -> f32 {
    let min_side = min(size.x, size.y);
    let min_corner = min(min(corners.x, corners.y), min(corners.z, corners.w));
    let max_corner = max(max(corners.x, corners.y), max(corners.z, corners.w));

    // A square quad whose corners are all exactly half its side is a circle;
    // keep knobs, color wells and status dots round. Radii above half are
    // clamped below and stay squircles.
    if (abs(size.x - size.y) < 1.0 && abs(min_corner - min_side) < 0.5 && abs(max_corner - min_side) < 0.5) {
        return length(p) - min_side;
    }

    var box_half = select(corners.yz, corners.xw, p.x > 0.0);
    var corner = select(box_half.y, box_half.x, p.y > 0.0);
    corner = min(corner, min_side);

    if (corner <= 0.0) {
        let q = abs(p) - size;
        return min(max(q.x, q.y), 0.0);
    }

    let q = abs(p) - size + corner;
    return min(max(q.x, q.y), 0.0) + superellipse_corner(q, corner);
}
