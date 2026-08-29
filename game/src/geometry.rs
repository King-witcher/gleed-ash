//! CPU-side mesh generators. Everything here is pure geometry: no GPU, no
//! `Result` — just parameters in, [`MeshData`] out.

use engine::{
    glam::prelude::*,
    model::{MeshData, Vertex},
};

/// Axis-aligned cube, one flat color per face.
///
/// Each face gets its own four vertices because the color is per-vertex and the
/// six faces meeting at a corner disagree on it. Winding is CCW seen from
/// outside, matching the pipeline's front face.
pub fn cube(center: Vec3, half_extent: f32) -> MeshData {
    let h = half_extent;

    // (four corners in CCW order seen from outside, face color)
    let faces = [
        // +X
        (
            [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]],
            [0.8, 0.2, 0.2],
        ),
        // -X
        (
            [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]],
            [0.4, 0.1, 0.1],
        ),
        // +Y
        (
            [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]],
            [0.2, 0.8, 0.2],
        ),
        // -Y
        (
            [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]],
            [0.1, 0.4, 0.1],
        ),
        // +Z
        (
            [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]],
            [0.2, 0.2, 0.8],
        ),
        // -Z
        (
            [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]],
            [0.1, 0.1, 0.4],
        ),
    ];

    let mut vertices = Vec::with_capacity(faces.len() * 4);
    let mut indices = Vec::with_capacity(faces.len() * 6);

    for (corners, color) in faces {
        let base = vertices.len() as u32;

        for corner in corners {
            vertices.push(Vertex {
                position: center + Vec3::from_array(corner),
                color: Vec3::from_array(color),
            });
        }

        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    MeshData { vertices, indices }
}

/// Truncated icosahedron — the football: 12 pentagons and 20 hexagons, every
/// vertex on the sphere of `radius`.
///
/// Built by cutting each icosahedron corner off at one third of every edge, so
/// each of the 20 triangles becomes a hexagon and each of the 12 corners becomes
/// a pentagon. The icosahedron's topology is derived from its corner positions
/// rather than a hand-written index table, which is also what gets the winding
/// right: every face comes out CCW seen from outside.
pub fn truncated_icosahedron(center: Vec3, radius: f32) -> MeshData {
    const PENTAGON_COLOR: [f32; 3] = [0.04, 0.04, 0.05];
    const HEXAGON_COLOR: [f32; 3] = [0.90, 0.90, 0.87];

    // Where the corner at `from` gets cut off the edge `from`-`to`.
    let cut = |from: Vec3, to: Vec3| from + (to - from) / 3.0;

    let corners = icosahedron_corners();
    let rings: Vec<[usize; 5]> = (0..corners.len())
        .map(|index| neighbour_ring(&corners, index))
        .collect();

    let mut faces: Vec<(Vec<Vec3>, [f32; 3])> = Vec::with_capacity(32);

    for (index, ring) in rings.iter().enumerate() {
        let a = corners[index];

        // The corner itself becomes a pentagon: cut every edge leaving it.
        faces.push((
            ring.iter().map(|&n| cut(a, corners[n])).collect(),
            PENTAGON_COLOR,
        ));

        // Two consecutive ring entries close an icosahedron triangle, which
        // becomes a hexagon. Each triangle turns up once per corner, so only its
        // lowest-numbered corner emits it.
        for i in 0..ring.len() {
            let (j, k) = (ring[i], ring[(i + 1) % ring.len()]);
            if index > j || index > k {
                continue;
            }

            let (b, c) = (corners[j], corners[k]);
            faces.push((
                vec![
                    cut(a, b),
                    cut(b, a),
                    cut(b, c),
                    cut(c, b),
                    cut(c, a),
                    cut(a, c),
                ],
                HEXAGON_COLOR,
            ));
        }
    }

    let mut vertices = Vec::with_capacity(180);
    let mut indices = Vec::with_capacity(348);

    for (polygon, color) in faces {
        let base = vertices.len() as u32;

        // Every vertex of a truncated icosahedron is the same distance from the
        // center, so normalizing here is a uniform scale onto the sphere.
        for point in &polygon {
            vertices.push(Vertex {
                position: center + point.normalize() * radius,
                color: Vec3::from_array(color),
            });
        }

        // The faces are convex, so a fan triangulates them and keeps the winding.
        for i in 1..polygon.len() as u32 - 1 {
            indices.extend_from_slice(&[base, base + i, base + i + 1]);
        }
    }

    MeshData { vertices, indices }
}

/// The 12 icosahedron corners: the cyclic permutations of (0, ±1, ±phi), which
/// put every edge at length 2.
fn icosahedron_corners() -> [Vec3; 12] {
    const PHI: f32 = 1.618_034;

    let mut corners = [Vec3::ZERO; 12];

    for (i, chunk) in corners.chunks_exact_mut(3).enumerate() {
        let s = if i & 1 == 0 { 1.0 } else { -1.0 };
        let t = if i & 2 == 0 { 1.0 } else { -1.0 };

        chunk[0] = Vec3::new(0.0, s, t * PHI);
        chunk[1] = Vec3::new(s, t * PHI, 0.0);
        chunk[2] = Vec3::new(s * PHI, 0.0, t);
    }

    corners
}

/// The five corners adjacent to `corners[index]` — an icosahedron corner has
/// exactly five, and they are its five nearest — ordered counter-clockwise as
/// seen from outside.
fn neighbour_ring(corners: &[Vec3; 12], index: usize) -> [usize; 5] {
    let corner = corners[index];

    // `u` and `v` span the plane facing outwards, with u x v = normal, so a
    // rising atan2 in that basis walks the ring counter-clockwise from outside.
    let normal = corner.normalize();
    let u = normal.any_orthonormal_vector();
    let v = normal.cross(u);
    let angle = |i: usize| {
        let offset = corners[i] - corner;
        offset.dot(v).atan2(offset.dot(u))
    };

    let mut ring: Vec<usize> = (0..corners.len()).filter(|&i| i != index).collect();
    ring.sort_by(|&a, &b| {
        corner
            .distance_squared(corners[a])
            .total_cmp(&corner.distance_squared(corners[b]))
    });
    ring.truncate(5);
    ring.sort_by(|&a, &b| angle(a).total_cmp(&angle(b)));

    ring.try_into().expect("truncated to five just above")
}
