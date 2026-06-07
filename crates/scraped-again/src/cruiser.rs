//! The cruiser's geometry — **game content** (M9): the engine's `ShipRenderer` draws
//! whatever mesh we hand it, so the actual ship lives here.
//!
//! A low-poly **faceted dart**: a sharp nose cone, a flat-sided tapering fuselage, swept
//! delta wings and a fin — built from flat triangles (each gets its own normal, for the
//! hard-edged faceted look). The **hull** is lit + palettised like the world; the
//! **nav-lights** are small emissive octahedra drawn after the palette so they pop. Authored
//! with +z forward, +y up; scaled by `ship::SHIP_SCALE` at draw time.

use brickmap::ship::Vertex;
use glam::Vec3;

/// The hull sits roughly around here; we flip each face normal to point away from it so the
/// faceted lighting is consistent regardless of triangle winding (the pass doesn't cull).
const CENTROID: Vec3 = Vec3::new(0.0, 0.0, -0.4);

fn v(pos: Vec3, color: [f32; 3], normal: Vec3, emissive: f32) -> Vertex {
    Vertex {
        pos: pos.to_array(),
        color,
        normal: normal.to_array(),
        emissive,
    }
}

/// Push a flat-shaded triangle (one outward normal for all three verts).
fn tri(out: &mut Vec<Vertex>, a: Vec3, b: Vec3, c: Vec3, color: [f32; 3]) {
    let mut n = (b - a).cross(c - a).normalize_or_zero();
    let face = (a + b + c) / 3.0;
    if n.dot(face - CENTROID) < 0.0 {
        n = -n; // make it face outward
    }
    out.push(v(a, color, n, 0.0));
    out.push(v(b, color, n, 0.0));
    out.push(v(c, color, n, 0.0));
}

/// Push a flat quad (a→b→c→d) as two triangles.
fn quad(out: &mut Vec<Vertex>, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [f32; 3]) {
    tri(out, a, b, c, color);
    tri(out, a, c, d, color);
}

/// A small emissive octahedron (a faceted glowing point) at `c`, radius `r`.
fn octa(out: &mut Vec<Vertex>, c: Vec3, r: f32, color: [f32; 3]) {
    let px = c + Vec3::X * r;
    let nx = c - Vec3::X * r;
    let py = c + Vec3::Y * r;
    let ny = c - Vec3::Y * r;
    let pz = c + Vec3::Z * r;
    let nz = c - Vec3::Z * r;
    let mut face = |a: Vec3, b: Vec3, d: Vec3| {
        let n = (b - a).cross(d - a).normalize_or_zero();
        out.push(v(a, color, n, 1.0));
        out.push(v(b, color, n, 1.0));
        out.push(v(d, color, n, 1.0));
    };
    for &(a, b) in &[(px, pz), (pz, nx), (nx, nz), (nz, px)] {
        face(py, a, b); // top fan
        face(ny, b, a); // bottom fan
    }
}

const HULL: [f32; 3] = [0.50, 0.55, 0.63];
const BELLY: [f32; 3] = [0.38, 0.41, 0.49];
const ENGINE: [f32; 3] = [0.16, 0.18, 0.26];

/// The lit hull (flat-faceted dart).
pub fn hull() -> Vec<Vertex> {
    let mut h = Vec::new();

    // Nose tip + the fuselage cross-section "rings" (flattened diamonds: top/right/bottom/left).
    let nose = Vec3::new(0.0, 0.05, 4.6);
    let (mt, mr, mb, ml) = (
        Vec3::new(0.0, 0.78, 0.2),
        Vec3::new(0.85, 0.02, 0.2),
        Vec3::new(0.0, -0.5, 0.2),
        Vec3::new(-0.85, 0.02, 0.2),
    );
    // Tail ring is broad (a fat engine block at the back), barely tapering from the mid ring.
    let (tt, tr, tb, tl) = (
        Vec3::new(0.0, 0.72, -3.4),
        Vec3::new(0.92, 0.06, -3.4),
        Vec3::new(0.0, -0.52, -3.4),
        Vec3::new(-0.92, 0.06, -3.4),
    );

    // Nose cone (4 angular facets).
    tri(&mut h, nose, mt, mr, HULL);
    tri(&mut h, nose, mr, mb, BELLY);
    tri(&mut h, nose, mb, ml, BELLY);
    tri(&mut h, nose, ml, mt, HULL);

    // Fuselage body (mid ring → tail ring), one quad per side.
    quad(&mut h, mt, mr, tr, tt, HULL); // top-right
    quad(&mut h, mr, mb, tb, tr, BELLY); // bottom-right
    quad(&mut h, mb, ml, tl, tb, BELLY); // bottom-left
    quad(&mut h, ml, mt, tt, tl, HULL); // top-left

    // Engine block (the flat tail cap).
    quad(&mut h, tt, tr, tb, tl, ENGINE);

    // Swept delta wings (flat triangles; the pass is winding-agnostic so they read both sides).
    tri(
        &mut h,
        Vec3::new(0.7, 0.0, 0.6),
        Vec3::new(0.6, 0.0, -2.0),
        Vec3::new(3.0, -0.05, -2.4),
        HULL,
    );
    tri(
        &mut h,
        Vec3::new(-0.7, 0.0, 0.6),
        Vec3::new(-0.6, 0.0, -2.0),
        Vec3::new(-3.0, -0.05, -2.4),
        HULL,
    );

    // Tail fin (a single vertical triangle).
    tri(
        &mut h,
        Vec3::new(0.0, 0.5, -1.8),
        Vec3::new(0.0, 0.5, -3.2),
        Vec3::new(0.0, 1.5, -2.9),
        HULL,
    );

    h
}

/// The emissive nav-lights: white-blue nose, amber tail, port-red / starboard-green wingtips.
/// Small octahedra so each reads as a bright faceted point after the palette.
pub fn lights() -> Vec<Vertex> {
    let mut l = Vec::new();
    octa(&mut l, Vec3::new(0.0, 0.1, 4.9), 0.16, [0.7, 0.9, 1.0]);
    octa(&mut l, Vec3::new(0.0, 0.28, -3.55), 0.18, [1.0, 0.7, 0.2]);
    octa(&mut l, Vec3::new(-3.0, -0.05, -2.4), 0.16, [1.0, 0.2, 0.2]);
    octa(&mut l, Vec3::new(3.0, -0.05, -2.4), 0.16, [0.2, 1.0, 0.3]);
    l
}
