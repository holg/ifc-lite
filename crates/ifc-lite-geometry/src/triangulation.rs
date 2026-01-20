// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Polygon triangulation utilities
//!
//! Wrapper around earcutr for 2D polygon triangulation.

use crate::{Error, Point2, Point3, Result, Vector3};

/// Check if a polygon is convex (all cross products have same sign)
#[inline]
fn is_convex(points: &[Point2<f64>]) -> bool {
    if points.len() < 3 {
        return false;
    }

    let n = points.len();
    let mut sign = 0i8;

    for i in 0..n {
        let p0 = &points[i];
        let p1 = &points[(i + 1) % n];
        let p2 = &points[(i + 2) % n];

        let cross = (p1.x - p0.x) * (p2.y - p1.y) - (p1.y - p0.y) * (p2.x - p1.x);

        if cross.abs() > 1e-10 {
            let current_sign = if cross > 0.0 { 1i8 } else { -1i8 };
            if sign == 0 {
                sign = current_sign;
            } else if sign != current_sign {
                return false;
            }
        }
    }

    true
}

/// Simple fan triangulation for convex polygons
#[inline]
fn fan_triangulate(n: usize) -> Vec<usize> {
    let mut indices = Vec::with_capacity((n - 2) * 3);
    for i in 1..n - 1 {
        indices.push(0);
        indices.push(i);
        indices.push(i + 1);
    }
    indices
}

/// Triangulate a simple polygon (no holes)
/// Returns triangle indices into the input points
#[inline]
pub fn triangulate_polygon(points: &[Point2<f64>]) -> Result<Vec<usize>> {
    let n = points.len();

    if n < 3 {
        return Err(Error::triangulation(
            "Need at least 3 points to triangulate",
        ));
    }

    // FAST PATH: Triangle - no triangulation needed
    if n == 3 {
        return Ok(vec![0, 1, 2]);
    }

    // FAST PATH: Quad - simple fan
    if n == 4 {
        return Ok(vec![0, 1, 2, 0, 2, 3]);
    }

    // FAST PATH: Convex polygon - use fan triangulation
    if n <= 8 && is_convex(points) {
        return Ok(fan_triangulate(n));
    }

    // Flatten points for earcutr
    let mut vertices = Vec::with_capacity(n * 2);
    for p in points {
        vertices.push(p.x);
        vertices.push(p.y);
    }

    // Triangulate using earcutr
    let indices = earcutr::earcut(&vertices, &[], 2)
        .map_err(|e| Error::triangulation(format!("{:?}", e)))?;

    Ok(indices)
}

/// Triangulate a polygon with holes
/// Returns triangle indices into the combined vertex array (outer + all holes)
#[inline]
pub fn triangulate_polygon_with_holes(
    outer: &[Point2<f64>],
    holes: &[Vec<Point2<f64>>],
) -> Result<Vec<usize>> {
    if outer.len() < 3 {
        return Err(Error::triangulation(
            "Need at least 3 points in outer boundary",
        ));
    }

    // Filter out empty or invalid holes
    let valid_holes: Vec<&Vec<Point2<f64>>> = holes.iter().filter(|h| h.len() >= 3).collect();

    if valid_holes.is_empty() {
        return triangulate_polygon(outer);
    }

    // Flatten vertices for earcutr
    let total_points: usize = outer.len() + valid_holes.iter().map(|h| h.len()).sum::<usize>();
    let mut vertices = Vec::with_capacity(total_points * 2);

    // Add outer boundary
    for p in outer {
        vertices.push(p.x);
        vertices.push(p.y);
    }

    // Add holes and track their start indices
    let mut hole_indices = Vec::with_capacity(valid_holes.len());
    for hole in valid_holes {
        hole_indices.push(vertices.len() / 2);
        for p in hole {
            vertices.push(p.x);
            vertices.push(p.y);
        }
    }

    // Triangulate using earcutr
    let indices = earcutr::earcut(&vertices, &hole_indices, 2)
        .map_err(|e| Error::triangulation(format!("{:?}", e)))?;

    Ok(indices)
}

/// Project 3D points onto a 2D plane defined by a normal
/// Returns 2D points and the coordinate system (u_axis, v_axis, origin)
#[inline]
pub fn project_to_2d(
    points_3d: &[Point3<f64>],
    normal: &Vector3<f64>,
) -> (Vec<Point2<f64>>, Vector3<f64>, Vector3<f64>, Point3<f64>) {
    if points_3d.is_empty() {
        return (
            Vec::new(),
            Vector3::zeros(),
            Vector3::zeros(),
            Point3::origin(),
        );
    }

    // Use first point as origin
    let origin = points_3d[0];

    // Create orthonormal basis on the plane
    let abs_x = normal.x.abs();
    let abs_y = normal.y.abs();
    let abs_z = normal.z.abs();

    let reference = if abs_x <= abs_y && abs_x <= abs_z {
        Vector3::new(1.0, 0.0, 0.0)
    } else if abs_y <= abs_z {
        Vector3::new(0.0, 1.0, 0.0)
    } else {
        Vector3::new(0.0, 0.0, 1.0)
    };

    let u_axis = normal.cross(&reference).normalize();
    let v_axis = normal.cross(&u_axis).normalize();

    // Project all points to 2D
    let points_2d = points_3d
        .iter()
        .map(|p| {
            let v = p - origin;
            Point2::new(v.dot(&u_axis), v.dot(&v_axis))
        })
        .collect();

    (points_2d, u_axis, v_axis, origin)
}

/// Project 3D points using an existing coordinate system
#[inline]
pub fn project_to_2d_with_basis(
    points_3d: &[Point3<f64>],
    u_axis: &Vector3<f64>,
    v_axis: &Vector3<f64>,
    origin: &Point3<f64>,
) -> Vec<Point2<f64>> {
    points_3d
        .iter()
        .map(|p| {
            let v = p - origin;
            Point2::new(v.dot(u_axis), v.dot(v_axis))
        })
        .collect()
}

/// Calculate the normal of a polygon from its vertices
#[inline]
pub fn calculate_polygon_normal(points: &[Point3<f64>]) -> Vector3<f64> {
    let n = points.len();

    if n < 3 {
        return Vector3::new(0.0, 0.0, 1.0);
    }

    // FAST PATH: Triangle or quad - use simple cross product
    if n <= 4 {
        let v1 = points[1] - points[0];
        let v2 = points[2] - points[0];
        let normal = v1.cross(&v2);
        let len = normal.norm();
        if len > 1e-10 {
            return normal / len;
        }
        if n == 4 {
            let v3 = points[3] - points[0];
            let normal = v2.cross(&v3);
            let len = normal.norm();
            if len > 1e-10 {
                return normal / len;
            }
        }
        return Vector3::new(0.0, 0.0, 1.0);
    }

    // Use Newell's method for robust normal calculation
    let mut normal = Vector3::<f64>::zeros();

    for i in 0..n {
        let current = &points[i];
        let next = &points[(i + 1) % n];

        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }

    let len = normal.norm();
    if len > 1e-10 {
        normal.normalize()
    } else {
        Vector3::new(0.0, 0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangulate_square() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];

        let indices = triangulate_polygon(&points).unwrap();
        assert_eq!(indices.len(), 6);
    }

    #[test]
    fn test_triangulate_triangle() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 1.0),
        ];

        let indices = triangulate_polygon(&points).unwrap();
        assert_eq!(indices.len(), 3);
    }

    #[test]
    fn test_calculate_polygon_normal() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];

        let normal = calculate_polygon_normal(&points);
        assert!((normal.z.abs() - 1.0).abs() < 0.001);
    }
}
