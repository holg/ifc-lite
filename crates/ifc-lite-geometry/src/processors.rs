// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Geometry Processors - Implementations for various IFC geometry types
//!
//! Each processor handles one or more types of IFC geometry representations.
//! Processors use the `EntityResolver` trait for entity lookups.

use crate::{
    extrusion::{apply_transform, extrude_profile},
    profile::Profile2D,
    Error, Mesh, Result, Vector3,
};
use ifc_lite_model::{DecodedEntity, EntityId, EntityResolver, IfcType};
use nalgebra::{Matrix4, Point2, Point3};

use super::router::GeometryProcessor;

/// ExtrudedAreaSolid processor
///
/// Handles IfcExtrudedAreaSolid - the most common IFC geometry type.
/// Extrudes 2D profiles along a direction vector.
pub struct ExtrudedAreaSolidProcessor;

impl ExtrudedAreaSolidProcessor {
    /// Create new processor
    pub fn new() -> Self {
        Self
    }

    /// Extract a 2D profile from an IFC profile definition
    fn extract_profile(
        &self,
        profile_entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Profile2D> {
        match profile_entity.ifc_type {
            IfcType::IfcRectangleProfileDef => self.extract_rectangle_profile(profile_entity),
            IfcType::IfcCircleProfileDef => self.extract_circle_profile(profile_entity),
            IfcType::IfcCircleHollowProfileDef => {
                self.extract_circle_hollow_profile(profile_entity)
            }
            IfcType::IfcArbitraryClosedProfileDef => {
                self.extract_arbitrary_profile(profile_entity, resolver)
            }
            IfcType::IfcArbitraryProfileDefWithVoids => {
                self.extract_arbitrary_profile_with_voids(profile_entity, resolver)
            }
            IfcType::IfcIShapeProfileDef => self.extract_i_shape_profile(profile_entity),
            IfcType::IfcLShapeProfileDef => self.extract_l_shape_profile(profile_entity),
            IfcType::IfcTShapeProfileDef => self.extract_t_shape_profile(profile_entity),
            _ => Err(Error::unsupported_type(format!(
                "Profile type {:?}",
                profile_entity.ifc_type
            ))),
        }
    }

    /// Extract rectangle profile
    fn extract_rectangle_profile(&self, entity: &DecodedEntity) -> Result<Profile2D> {
        // XDim at index 3, YDim at index 4
        let x_dim = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing XDim"))?;
        let y_dim = entity
            .get_float(4)
            .ok_or_else(|| Error::invalid_attribute(4, "Missing YDim"))?;

        Ok(Profile2D::rectangle(x_dim, y_dim))
    }

    /// Extract circle profile
    fn extract_circle_profile(&self, entity: &DecodedEntity) -> Result<Profile2D> {
        // Radius at index 3
        let radius = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing Radius"))?;

        Ok(Profile2D::circle(radius, None))
    }

    /// Extract hollow circle profile
    fn extract_circle_hollow_profile(&self, entity: &DecodedEntity) -> Result<Profile2D> {
        // Radius at index 3, WallThickness at index 4
        let radius = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing Radius"))?;
        let wall_thickness = entity
            .get_float(4)
            .ok_or_else(|| Error::invalid_attribute(4, "Missing WallThickness"))?;

        let inner_radius = radius - wall_thickness;
        if inner_radius <= 0.0 {
            return Err(Error::geometry("Invalid hollow circle: inner radius <= 0"));
        }

        // Create outer circle
        let mut profile = Profile2D::circle(radius, None);

        // Add inner circle as hole
        let segments = crate::profile::calculate_circle_segments(inner_radius);
        let mut hole = Vec::with_capacity(segments);
        for i in 0..segments {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (segments as f64);
            hole.push(Point2::new(
                inner_radius * angle.cos(),
                inner_radius * angle.sin(),
            ));
        }
        hole.reverse(); // Clockwise for hole
        profile.add_hole(hole);

        Ok(profile)
    }

    /// Extract arbitrary closed profile
    fn extract_arbitrary_profile(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Profile2D> {
        // OuterCurve at index 2
        let curve_id = entity
            .get_ref(2)
            .ok_or_else(|| Error::invalid_attribute(2, "Missing OuterCurve"))?;

        let points = self.extract_polyline_points(curve_id, resolver)?;
        if points.len() < 3 {
            return Err(Error::profile("Profile must have at least 3 points"));
        }

        Ok(Profile2D::new(points))
    }

    /// Extract arbitrary profile with voids (holes)
    fn extract_arbitrary_profile_with_voids(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Profile2D> {
        // OuterCurve at index 2
        let outer_curve_id = entity
            .get_ref(2)
            .ok_or_else(|| Error::invalid_attribute(2, "Missing OuterCurve"))?;

        let outer_points = self.extract_polyline_points(outer_curve_id, resolver)?;
        if outer_points.len() < 3 {
            return Err(Error::profile("Outer profile must have at least 3 points"));
        }

        let mut profile = Profile2D::new(outer_points);

        // InnerCurves at index 3
        if let Some(inner_curves) = entity.get(3) {
            if let Some(list) = inner_curves.as_list() {
                for curve_ref in list {
                    if let Some(curve_id) = curve_ref.as_entity_ref() {
                        if let Ok(hole_points) = self.extract_polyline_points(curve_id, resolver) {
                            if hole_points.len() >= 3 {
                                profile.add_hole(hole_points);
                            }
                        }
                    }
                }
            }
        }

        Ok(profile)
    }

    /// Extract I-shape profile
    fn extract_i_shape_profile(&self, entity: &DecodedEntity) -> Result<Profile2D> {
        // Attributes: OverallWidth(3), OverallDepth(4), WebThickness(5), FlangeThickness(6)
        let width = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing OverallWidth"))?;
        let depth = entity
            .get_float(4)
            .ok_or_else(|| Error::invalid_attribute(4, "Missing OverallDepth"))?;
        let web_thickness = entity
            .get_float(5)
            .ok_or_else(|| Error::invalid_attribute(5, "Missing WebThickness"))?;
        let flange_thickness = entity
            .get_float(6)
            .ok_or_else(|| Error::invalid_attribute(6, "Missing FlangeThickness"))?;

        // Create I-shape profile
        let hw = width / 2.0;
        let hd = depth / 2.0;
        let hwt = web_thickness / 2.0;
        let ft = flange_thickness;

        let points = vec![
            Point2::new(-hw, -hd),
            Point2::new(hw, -hd),
            Point2::new(hw, -hd + ft),
            Point2::new(hwt, -hd + ft),
            Point2::new(hwt, hd - ft),
            Point2::new(hw, hd - ft),
            Point2::new(hw, hd),
            Point2::new(-hw, hd),
            Point2::new(-hw, hd - ft),
            Point2::new(-hwt, hd - ft),
            Point2::new(-hwt, -hd + ft),
            Point2::new(-hw, -hd + ft),
        ];

        Ok(Profile2D::new(points))
    }

    /// Extract L-shape profile
    fn extract_l_shape_profile(&self, entity: &DecodedEntity) -> Result<Profile2D> {
        // Attributes: Depth(3), Width(4), Thickness(5)
        let depth = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing Depth"))?;
        let width = entity
            .get_float(4)
            .ok_or_else(|| Error::invalid_attribute(4, "Missing Width"))?;
        let thickness = entity
            .get_float(5)
            .ok_or_else(|| Error::invalid_attribute(5, "Missing Thickness"))?;

        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(width, 0.0),
            Point2::new(width, thickness),
            Point2::new(thickness, thickness),
            Point2::new(thickness, depth),
            Point2::new(0.0, depth),
        ];

        Ok(Profile2D::new(points))
    }

    /// Extract T-shape profile
    fn extract_t_shape_profile(&self, entity: &DecodedEntity) -> Result<Profile2D> {
        // Attributes: Depth(3), FlangeWidth(4), WebThickness(5), FlangeThickness(6)
        let depth = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing Depth"))?;
        let flange_width = entity
            .get_float(4)
            .ok_or_else(|| Error::invalid_attribute(4, "Missing FlangeWidth"))?;
        let web_thickness = entity
            .get_float(5)
            .ok_or_else(|| Error::invalid_attribute(5, "Missing WebThickness"))?;
        let flange_thickness = entity
            .get_float(6)
            .ok_or_else(|| Error::invalid_attribute(6, "Missing FlangeThickness"))?;

        let hfw = flange_width / 2.0;
        let hwt = web_thickness / 2.0;

        let points = vec![
            Point2::new(-hfw, 0.0),
            Point2::new(hfw, 0.0),
            Point2::new(hfw, flange_thickness),
            Point2::new(hwt, flange_thickness),
            Point2::new(hwt, depth),
            Point2::new(-hwt, depth),
            Point2::new(-hwt, flange_thickness),
            Point2::new(-hfw, flange_thickness),
        ];

        Ok(Profile2D::new(points))
    }

    /// Extract points from a polyline
    fn extract_polyline_points(
        &self,
        curve_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Result<Vec<Point2<f64>>> {
        let curve = resolver
            .get(curve_id)
            .ok_or_else(|| Error::entity_not_found(curve_id.0))?;

        match curve.ifc_type {
            IfcType::IfcPolyline => {
                // Points at index 0
                let points_list = curve
                    .get(0)
                    .and_then(|v| v.as_list())
                    .ok_or_else(|| Error::invalid_attribute(0, "Missing Points"))?;

                let mut points = Vec::with_capacity(points_list.len());
                for point_ref in points_list {
                    if let Some(point_id) = point_ref.as_entity_ref() {
                        if let Some(point_entity) = resolver.get(point_id) {
                            if let Some(coords) = point_entity.get(0).and_then(|v| v.as_list()) {
                                let x = coords.first().and_then(|v| v.as_float()).unwrap_or(0.0);
                                let y = coords.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
                                points.push(Point2::new(x, y));
                            }
                        }
                    }
                }

                // Remove duplicate last point if closed
                if points.len() > 1 {
                    let first = points.first().unwrap();
                    let last = points.last().unwrap();
                    if (first.x - last.x).abs() < 1e-10 && (first.y - last.y).abs() < 1e-10 {
                        points.pop();
                    }
                }

                Ok(points)
            }
            IfcType::IfcIndexedPolyCurve => {
                // Handle indexed poly curve
                self.extract_indexed_poly_curve_points(&curve, resolver)
            }
            _ => Err(Error::unsupported_type(format!(
                "Curve type {:?}",
                curve.ifc_type
            ))),
        }
    }

    /// Extract points from an indexed poly curve
    fn extract_indexed_poly_curve_points(
        &self,
        curve: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Vec<Point2<f64>>> {
        // Points at index 0 (IfcCartesianPointList2D)
        let points_ref_id = curve
            .get_ref(0)
            .ok_or_else(|| Error::invalid_attribute(0, "Missing Points"))?;

        let points_list = resolver
            .get(points_ref_id)
            .ok_or_else(|| Error::entity_not_found(points_ref_id.0))?;

        // CoordList at index 0
        let coords = points_list
            .get(0)
            .and_then(|v| v.as_list())
            .ok_or_else(|| Error::invalid_attribute(0, "Missing CoordList"))?;

        let mut points = Vec::with_capacity(coords.len());
        for coord in coords {
            if let Some(coord_list) = coord.as_list() {
                let x = coord_list.first().and_then(|v| v.as_float()).unwrap_or(0.0);
                let y = coord_list.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
                points.push(Point2::new(x, y));
            }
        }

        Ok(points)
    }

    /// Extract IfcAxis2Placement3D transform
    fn extract_position_transform(
        &self,
        position_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<Matrix4<f64>> {
        let position = resolver.get(position_id)?;

        if position.ifc_type != IfcType::IfcAxis2Placement3D {
            return None;
        }

        // Location (index 0)
        let location_id = position.get_ref(0)?;
        let location = resolver.get(location_id)?;
        let coords = location.get(0)?.as_list()?;

        let x = coords.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let y = coords.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        let z = coords.get(2).and_then(|v| v.as_float()).unwrap_or(0.0);

        // Axis (index 1) - Z direction
        let axis = position
            .get_ref(1)
            .and_then(|id| self.extract_direction(id, resolver))
            .unwrap_or_else(|| Vector3::new(0.0, 0.0, 1.0));

        // RefDirection (index 2) - X direction
        let ref_dir = position
            .get_ref(2)
            .and_then(|id| self.extract_direction(id, resolver))
            .unwrap_or_else(|| Vector3::new(1.0, 0.0, 0.0));

        // Build orthonormal basis
        let z_axis = axis.normalize();
        let x_axis = ref_dir.normalize();
        let y_axis = z_axis.cross(&x_axis).normalize();
        let x_axis = y_axis.cross(&z_axis).normalize();

        Some(Matrix4::new(
            x_axis.x, y_axis.x, z_axis.x, x, x_axis.y, y_axis.y, z_axis.y, y, x_axis.z, y_axis.z,
            z_axis.z, z, 0.0, 0.0, 0.0, 1.0,
        ))
    }

    /// Extract direction vector
    fn extract_direction(
        &self,
        dir_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<Vector3<f64>> {
        let direction = resolver.get(dir_id)?;
        let ratios = direction.get(0)?.as_list()?;

        let x = ratios.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let y = ratios.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        let z = ratios.get(2).and_then(|v| v.as_float()).unwrap_or(1.0);

        Some(Vector3::new(x, y, z))
    }
}

impl Default for ExtrudedAreaSolidProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryProcessor for ExtrudedAreaSolidProcessor {
    fn process(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
        _unit_scale: f64,
    ) -> Result<Mesh> {
        // IfcExtrudedAreaSolid attributes:
        // 0: SweptArea (IfcProfileDef)
        // 1: Position (IfcAxis2Placement3D)
        // 2: ExtrudedDirection (IfcDirection)
        // 3: Depth (IfcPositiveLengthMeasure)

        // Get profile
        let profile_id = entity
            .get_ref(0)
            .ok_or_else(|| Error::invalid_attribute(0, "Missing SweptArea"))?;

        let profile_entity = resolver
            .get(profile_id)
            .ok_or_else(|| Error::entity_not_found(profile_id.0))?;

        let profile = self.extract_profile(&profile_entity, resolver)?;

        if profile.outer.is_empty() {
            return Ok(Mesh::new());
        }

        // Get extrusion direction
        let direction_id = entity
            .get_ref(2)
            .ok_or_else(|| Error::invalid_attribute(2, "Missing ExtrudedDirection"))?;

        let direction_entity = resolver
            .get(direction_id)
            .ok_or_else(|| Error::entity_not_found(direction_id.0))?;

        let ratios = direction_entity
            .get(0)
            .and_then(|v| v.as_list())
            .ok_or_else(|| Error::invalid_attribute(0, "Missing direction ratios"))?;

        let dir_x = ratios.first().and_then(|v| v.as_float()).unwrap_or(0.0);
        let dir_y = ratios.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
        let dir_z = ratios.get(2).and_then(|v| v.as_float()).unwrap_or(1.0);

        let direction = Vector3::new(dir_x, dir_y, dir_z).normalize();

        // Get depth
        let depth = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing Depth"))?;

        // Determine transform based on extrusion direction
        let extrusion_transform = if direction.x.abs() < 0.001 && direction.y.abs() < 0.001 {
            // Extrusion along Z axis (common case)
            if direction.z < 0.0 {
                // Negative Z - shift down
                Some(Matrix4::new_translation(&Vector3::new(0.0, 0.0, -depth)))
            } else {
                None
            }
        } else {
            // Non-Z-aligned extrusion - compute rotation
            let z_axis = Vector3::new(0.0, 0.0, 1.0);
            let rotation_axis = z_axis.cross(&direction);
            let rotation_angle = z_axis.dot(&direction).acos();

            if rotation_axis.norm() > 1e-10 {
                Some(Matrix4::new_rotation(rotation_axis.normalize() * rotation_angle))
            } else {
                None
            }
        };

        // Extrude profile
        let mut mesh = extrude_profile(&profile, depth, extrusion_transform)?;

        // Apply position transform
        if let Some(position_id) = entity.get_ref(1) {
            if let Some(transform) = self.extract_position_transform(position_id, resolver) {
                apply_transform(&mut mesh, &transform);
            }
        }

        Ok(mesh)
    }

    fn supported_types(&self) -> Vec<IfcType> {
        vec![IfcType::IfcExtrudedAreaSolid]
    }
}

/// TriangulatedFaceSet processor
///
/// Handles IfcTriangulatedFaceSet - explicit triangle meshes (IFC4+)
pub struct TriangulatedFaceSetProcessor;

impl TriangulatedFaceSetProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TriangulatedFaceSetProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryProcessor for TriangulatedFaceSetProcessor {
    fn process(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
        _unit_scale: f64,
    ) -> Result<Mesh> {
        // IfcTriangulatedFaceSet attributes:
        // 0: Coordinates (IfcCartesianPointList3D)
        // 1: Normals (optional)
        // 2: Closed (optional)
        // 3: CoordIndex (list of list of IfcPositiveInteger)

        // Get coordinate entity reference
        let coord_id = entity
            .get_ref(0)
            .ok_or_else(|| Error::invalid_attribute(0, "Missing Coordinates"))?;

        let coord_entity = resolver
            .get(coord_id)
            .ok_or_else(|| Error::entity_not_found(coord_id.0))?;

        // IfcCartesianPointList3D has CoordList at index 0
        let coord_list = coord_entity
            .get(0)
            .and_then(|v| v.as_list())
            .ok_or_else(|| Error::invalid_attribute(0, "Missing CoordList"))?;

        // Parse coordinates
        let mut positions = Vec::with_capacity(coord_list.len() * 3);
        for coord in coord_list {
            if let Some(point) = coord.as_list() {
                let x = point.first().and_then(|v| v.as_float()).unwrap_or(0.0);
                let y = point.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
                let z = point.get(2).and_then(|v| v.as_float()).unwrap_or(0.0);
                positions.push(x as f32);
                positions.push(y as f32);
                positions.push(z as f32);
            }
        }

        // Get face indices (CoordIndex at index 3)
        let indices_attr = entity
            .get(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing CoordIndex"))?;

        let face_list = indices_attr
            .as_list()
            .ok_or_else(|| Error::invalid_attribute(3, "Expected list for CoordIndex"))?;

        // Parse indices - IFC uses 1-based indexing
        let mut indices = Vec::with_capacity(face_list.len() * 3);
        for face in face_list {
            if let Some(triangle) = face.as_list() {
                if triangle.len() >= 3 {
                    // IFC uses 1-based indexing, convert to 0-based
                    let i0 = triangle
                        .first()
                        .and_then(|v| v.as_integer())
                        .unwrap_or(1) as u32
                        - 1;
                    let i1 = triangle
                        .get(1)
                        .and_then(|v| v.as_integer())
                        .unwrap_or(1) as u32
                        - 1;
                    let i2 = triangle
                        .get(2)
                        .and_then(|v| v.as_integer())
                        .unwrap_or(1) as u32
                        - 1;
                    indices.push(i0);
                    indices.push(i1);
                    indices.push(i2);
                }
            }
        }

        Ok(Mesh {
            positions,
            normals: Vec::new(),
            indices,
        })
    }

    fn supported_types(&self) -> Vec<IfcType> {
        vec![IfcType::IfcTriangulatedFaceSet]
    }
}

/// FacetedBrep processor
///
/// Handles IfcFacetedBrep - explicit mesh with faces.
/// Supports faces with inner bounds (holes).
pub struct FacetedBrepProcessor;

impl FacetedBrepProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Extract polygon points from a loop entity
    fn extract_loop_points(
        &self,
        loop_id: EntityId,
        resolver: &dyn EntityResolver,
    ) -> Option<Vec<Point3<f64>>> {
        let loop_entity = resolver.get(loop_id)?;

        // IfcPolyLoop has Polygon attribute at index 0
        let polygon_attr = loop_entity.get(0)?;
        let point_refs = polygon_attr.as_list()?;

        let mut points = Vec::with_capacity(point_refs.len());

        for point_ref in point_refs {
            let point_id = point_ref.as_entity_ref()?;
            let point_entity = resolver.get(point_id)?;

            // IfcCartesianPoint has Coordinates at index 0
            let coords = point_entity.get(0)?.as_list()?;

            let x = coords.first().and_then(|v| v.as_float()).unwrap_or(0.0);
            let y = coords.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
            let z = coords.get(2).and_then(|v| v.as_float()).unwrap_or(0.0);

            points.push(Point3::new(x, y, z));
        }

        if points.len() >= 3 {
            Some(points)
        } else {
            None
        }
    }

    /// Triangulate a face (supports holes)
    fn triangulate_face(
        &self,
        outer_points: &[Point3<f64>],
        hole_points: &[Vec<Point3<f64>>],
    ) -> (Vec<f32>, Vec<u32>) {
        let n = outer_points.len();

        // Fast path: triangle without holes
        if n == 3 && hole_points.is_empty() {
            let mut positions = Vec::with_capacity(9);
            for point in outer_points {
                positions.push(point.x as f32);
                positions.push(point.y as f32);
                positions.push(point.z as f32);
            }
            return (positions, vec![0, 1, 2]);
        }

        // Fast path: quad without holes
        if n == 4 && hole_points.is_empty() {
            let mut positions = Vec::with_capacity(12);
            for point in outer_points {
                positions.push(point.x as f32);
                positions.push(point.y as f32);
                positions.push(point.z as f32);
            }
            return (positions, vec![0, 1, 2, 0, 2, 3]);
        }

        // Complex polygon or has holes - use triangulation
        use crate::triangulation::{
            calculate_polygon_normal, project_to_2d, project_to_2d_with_basis,
            triangulate_polygon_with_holes,
        };

        let normal = calculate_polygon_normal(outer_points);
        let (outer_2d, u_axis, v_axis, origin) = project_to_2d(outer_points, &normal);

        let holes_2d: Vec<Vec<Point2<f64>>> = hole_points
            .iter()
            .map(|hole| project_to_2d_with_basis(hole, &u_axis, &v_axis, &origin))
            .collect();

        let tri_indices = match triangulate_polygon_with_holes(&outer_2d, &holes_2d) {
            Ok(idx) => idx,
            Err(_) => {
                // Fallback to simple fan triangulation
                let mut positions = Vec::with_capacity(n * 3);
                for point in outer_points {
                    positions.push(point.x as f32);
                    positions.push(point.y as f32);
                    positions.push(point.z as f32);
                }
                let mut indices = Vec::with_capacity((n - 2) * 3);
                for i in 1..n - 1 {
                    indices.push(0);
                    indices.push(i as u32);
                    indices.push(i as u32 + 1);
                }
                return (positions, indices);
            }
        };

        // Combine all 3D points (outer + holes)
        let mut all_points: Vec<&Point3<f64>> = outer_points.iter().collect();
        for hole in hole_points {
            all_points.extend(hole.iter());
        }

        let mut positions = Vec::with_capacity(all_points.len() * 3);
        for point in &all_points {
            positions.push(point.x as f32);
            positions.push(point.y as f32);
            positions.push(point.z as f32);
        }

        let indices: Vec<u32> = tri_indices.iter().map(|&i| i as u32).collect();

        (positions, indices)
    }
}

impl Default for FacetedBrepProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryProcessor for FacetedBrepProcessor {
    fn process(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
        _unit_scale: f64,
    ) -> Result<Mesh> {
        // IfcFacetedBrep attributes:
        // 0: Outer (IfcClosedShell)

        let shell_id = entity
            .get_ref(0)
            .ok_or_else(|| Error::invalid_attribute(0, "Missing Outer shell"))?;

        let shell_entity = resolver
            .get(shell_id)
            .ok_or_else(|| Error::entity_not_found(shell_id.0))?;

        // IfcClosedShell has CfsFaces at index 0
        let faces = shell_entity
            .get(0)
            .and_then(|v| v.as_list())
            .ok_or_else(|| Error::invalid_attribute(0, "Missing CfsFaces"))?;

        let mut all_positions = Vec::new();
        let mut all_indices = Vec::new();

        for face_ref in faces {
            let face_id = match face_ref.as_entity_ref() {
                Some(id) => id,
                None => continue,
            };

            let face_entity = match resolver.get(face_id) {
                Some(e) => e,
                None => continue,
            };

            // IfcFace has Bounds at index 0
            let bounds = match face_entity.get(0).and_then(|v| v.as_list()) {
                Some(b) => b,
                None => continue,
            };

            let mut outer_points: Option<Vec<Point3<f64>>> = None;
            let mut hole_points: Vec<Vec<Point3<f64>>> = Vec::new();

            for bound_ref in bounds {
                let bound_id = match bound_ref.as_entity_ref() {
                    Some(id) => id,
                    None => continue,
                };

                let bound_entity = match resolver.get(bound_id) {
                    Some(e) => e,
                    None => continue,
                };

                // Get loop reference (index 0)
                let loop_id = match bound_entity.get_ref(0) {
                    Some(id) => id,
                    None => continue,
                };

                // Get orientation (index 1)
                let orientation = bound_entity
                    .get(1)
                    .map(|v| match v {
                        ifc_lite_model::AttributeValue::Enum(e) => e != "F" && e != ".F.",
                        ifc_lite_model::AttributeValue::Bool(b) => *b,
                        _ => true,
                    })
                    .unwrap_or(true);

                let mut points = match self.extract_loop_points(loop_id, resolver) {
                    Some(p) => p,
                    None => continue,
                };

                if !orientation {
                    points.reverse();
                }

                let is_outer = bound_entity.ifc_type == IfcType::IfcFaceOuterBound;

                if is_outer || outer_points.is_none() {
                    if outer_points.is_some() && is_outer {
                        if let Some(prev_outer) = outer_points.take() {
                            hole_points.push(prev_outer);
                        }
                    }
                    outer_points = Some(points);
                } else {
                    hole_points.push(points);
                }
            }

            if let Some(outer) = outer_points {
                let base_idx = (all_positions.len() / 3) as u32;
                let (positions, indices) = self.triangulate_face(&outer, &hole_points);

                all_positions.extend(positions);
                for idx in indices {
                    all_indices.push(base_idx + idx);
                }
            }
        }

        Ok(Mesh {
            positions: all_positions,
            normals: Vec::new(),
            indices: all_indices,
        })
    }

    fn supported_types(&self) -> Vec<IfcType> {
        vec![IfcType::IfcFacetedBrep]
    }
}

/// SweptDiskSolid processor
///
/// Handles IfcSweptDiskSolid - sweeps a circular profile along a curve.
pub struct SweptDiskSolidProcessor;

impl SweptDiskSolidProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Extract points from a curve
    fn get_curve_points(
        &self,
        curve: &DecodedEntity,
        resolver: &dyn EntityResolver,
    ) -> Result<Vec<Point3<f64>>> {
        match curve.ifc_type {
            IfcType::IfcPolyline => {
                // IfcPolyline: Points at index 0
                let points_list = curve
                    .get(0)
                    .and_then(|v| v.as_list())
                    .ok_or_else(|| Error::invalid_attribute(0, "Missing Points"))?;

                let mut points = Vec::with_capacity(points_list.len());
                for point_ref in points_list {
                    if let Some(point_id) = point_ref.as_entity_ref() {
                        if let Some(point_entity) = resolver.get(point_id) {
                            if let Some(coords) = point_entity.get(0).and_then(|v| v.as_list()) {
                                let x = coords.first().and_then(|v| v.as_float()).unwrap_or(0.0);
                                let y = coords.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
                                let z = coords.get(2).and_then(|v| v.as_float()).unwrap_or(0.0);
                                points.push(Point3::new(x, y, z));
                            }
                        }
                    }
                }
                Ok(points)
            }
            IfcType::IfcIndexedPolyCurve => {
                // IfcIndexedPolyCurve: Points (IfcCartesianPointList3D) at index 0
                let points_ref_id = curve
                    .get_ref(0)
                    .ok_or_else(|| Error::invalid_attribute(0, "Missing Points"))?;

                let points_entity = resolver
                    .get(points_ref_id)
                    .ok_or_else(|| Error::entity_not_found(points_ref_id.0))?;

                // IfcCartesianPointList3D: CoordList at index 0
                let coord_list = points_entity
                    .get(0)
                    .and_then(|v| v.as_list())
                    .ok_or_else(|| Error::invalid_attribute(0, "Missing CoordList"))?;

                let mut points = Vec::with_capacity(coord_list.len());
                for coord in coord_list {
                    if let Some(point) = coord.as_list() {
                        let x = point.first().and_then(|v| v.as_float()).unwrap_or(0.0);
                        let y = point.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
                        let z = point.get(2).and_then(|v| v.as_float()).unwrap_or(0.0);
                        points.push(Point3::new(x, y, z));
                    }
                }
                Ok(points)
            }
            _ => Err(Error::unsupported_type(format!(
                "Curve type {:?}",
                curve.ifc_type
            ))),
        }
    }
}

impl Default for SweptDiskSolidProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryProcessor for SweptDiskSolidProcessor {
    fn process(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
        _unit_scale: f64,
    ) -> Result<Mesh> {
        // IfcSweptDiskSolid attributes:
        // 0: Directrix (IfcCurve)
        // 1: Radius
        // 2: InnerRadius (optional)
        // 3: StartParam (optional)
        // 4: EndParam (optional)

        let directrix_id = entity
            .get_ref(0)
            .ok_or_else(|| Error::invalid_attribute(0, "Missing Directrix"))?;

        let directrix = resolver
            .get(directrix_id)
            .ok_or_else(|| Error::entity_not_found(directrix_id.0))?;

        let radius = entity
            .get_float(1)
            .ok_or_else(|| Error::invalid_attribute(1, "Missing Radius"))?;

        let curve_points = self.get_curve_points(&directrix, resolver)?;

        if curve_points.len() < 2 {
            return Ok(Mesh::new());
        }

        // Generate tube mesh
        let segments = 12;
        let mut positions = Vec::new();
        let mut indices = Vec::new();

        for i in 0..curve_points.len() {
            let p = curve_points[i];

            // Calculate tangent
            let tangent = if i == 0 {
                (curve_points[1] - curve_points[0]).normalize()
            } else if i == curve_points.len() - 1 {
                (curve_points[i] - curve_points[i - 1]).normalize()
            } else {
                ((curve_points[i + 1] - curve_points[i - 1]) / 2.0).normalize()
            };

            // Create perpendicular vectors
            let up = if tangent.x.abs() < 0.9 {
                Vector3::new(1.0, 0.0, 0.0)
            } else {
                Vector3::new(0.0, 1.0, 0.0)
            };

            let perp1 = tangent.cross(&up).normalize();
            let perp2 = tangent.cross(&perp1).normalize();

            // Create ring of vertices
            for j in 0..segments {
                let angle = 2.0 * std::f64::consts::PI * j as f64 / segments as f64;
                let offset = perp1 * (radius * angle.cos()) + perp2 * (radius * angle.sin());
                let vertex = p + offset;

                positions.push(vertex.x as f32);
                positions.push(vertex.y as f32);
                positions.push(vertex.z as f32);
            }

            // Create triangles connecting this ring to the next
            if i < curve_points.len() - 1 {
                let base = (i * segments) as u32;
                let next_base = ((i + 1) * segments) as u32;

                for j in 0..segments {
                    let j_next = (j + 1) % segments;

                    indices.push(base + j as u32);
                    indices.push(next_base + j as u32);
                    indices.push(next_base + j_next as u32);

                    indices.push(base + j as u32);
                    indices.push(next_base + j_next as u32);
                    indices.push(base + j_next as u32);
                }
            }
        }

        // Add end caps
        let center_idx = (positions.len() / 3) as u32;
        let start = curve_points[0];
        positions.push(start.x as f32);
        positions.push(start.y as f32);
        positions.push(start.z as f32);

        for j in 0..segments {
            let j_next = (j + 1) % segments;
            indices.push(center_idx);
            indices.push(j_next as u32);
            indices.push(j as u32);
        }

        let end_center_idx = (positions.len() / 3) as u32;
        let end_base = ((curve_points.len() - 1) * segments) as u32;
        let end = curve_points[curve_points.len() - 1];
        positions.push(end.x as f32);
        positions.push(end.y as f32);
        positions.push(end.z as f32);

        for j in 0..segments {
            let j_next = (j + 1) % segments;
            indices.push(end_center_idx);
            indices.push(end_base + j as u32);
            indices.push(end_base + j_next as u32);
        }

        Ok(Mesh {
            positions,
            normals: Vec::new(),
            indices,
        })
    }

    fn supported_types(&self) -> Vec<IfcType> {
        vec![IfcType::IfcSweptDiskSolid]
    }
}

/// RevolvedAreaSolid processor
///
/// Handles IfcRevolvedAreaSolid - rotates a 2D profile around an axis.
pub struct RevolvedAreaSolidProcessor;

impl RevolvedAreaSolidProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Extract 2D profile points
    fn extract_profile(&self, profile: &DecodedEntity, resolver: &dyn EntityResolver) -> Result<Vec<Point2<f64>>> {
        match profile.ifc_type {
            IfcType::IfcRectangleProfileDef => {
                let x_dim = profile.get_float(3).unwrap_or(1.0);
                let y_dim = profile.get_float(4).unwrap_or(1.0);
                let hx = x_dim / 2.0;
                let hy = y_dim / 2.0;
                Ok(vec![
                    Point2::new(-hx, -hy),
                    Point2::new(hx, -hy),
                    Point2::new(hx, hy),
                    Point2::new(-hx, hy),
                ])
            }
            IfcType::IfcCircleProfileDef => {
                let radius = profile.get_float(3).unwrap_or(1.0);
                let segments = crate::profile::calculate_circle_segments(radius);
                let mut points = Vec::with_capacity(segments);
                for i in 0..segments {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / (segments as f64);
                    points.push(Point2::new(radius * angle.cos(), radius * angle.sin()));
                }
                Ok(points)
            }
            IfcType::IfcArbitraryClosedProfileDef => {
                let curve_id = profile
                    .get_ref(2)
                    .ok_or_else(|| Error::invalid_attribute(2, "Missing OuterCurve"))?;

                let curve = resolver
                    .get(curve_id)
                    .ok_or_else(|| Error::entity_not_found(curve_id.0))?;

                self.extract_curve_2d_points(&curve, resolver)
            }
            _ => Err(Error::unsupported_type(format!("{:?}", profile.ifc_type))),
        }
    }

    fn extract_curve_2d_points(&self, curve: &DecodedEntity, resolver: &dyn EntityResolver) -> Result<Vec<Point2<f64>>> {
        if curve.ifc_type == IfcType::IfcPolyline {
            let points_list = curve
                .get(0)
                .and_then(|v| v.as_list())
                .ok_or_else(|| Error::invalid_attribute(0, "Missing Points"))?;

            let mut points = Vec::with_capacity(points_list.len());
            for point_ref in points_list {
                if let Some(point_id) = point_ref.as_entity_ref() {
                    if let Some(point_entity) = resolver.get(point_id) {
                        if let Some(coords) = point_entity.get(0).and_then(|v| v.as_list()) {
                            let x = coords.first().and_then(|v| v.as_float()).unwrap_or(0.0);
                            let y = coords.get(1).and_then(|v| v.as_float()).unwrap_or(0.0);
                            points.push(Point2::new(x, y));
                        }
                    }
                }
            }
            Ok(points)
        } else {
            Err(Error::unsupported_type(format!("{:?}", curve.ifc_type)))
        }
    }

    fn parse_axis_location(&self, axis: &DecodedEntity, resolver: &dyn EntityResolver) -> Result<Point3<f64>> {
        let loc_id = axis.get_ref(0)
            .ok_or_else(|| Error::invalid_attribute(0, "Missing Location"))?;

        let loc = resolver.get(loc_id)
            .ok_or_else(|| Error::entity_not_found(loc_id.0))?;

        let coords = loc.get(0).and_then(|v| v.as_list())
            .ok_or_else(|| Error::invalid_attribute(0, "Missing coordinates"))?;

        Ok(Point3::new(
            coords.first().and_then(|v| v.as_float()).unwrap_or(0.0),
            coords.get(1).and_then(|v| v.as_float()).unwrap_or(0.0),
            coords.get(2).and_then(|v| v.as_float()).unwrap_or(0.0),
        ))
    }

    fn parse_axis_direction(&self, axis: &DecodedEntity, resolver: &dyn EntityResolver) -> Vector3<f64> {
        if let Some(dir_id) = axis.get_ref(1) {
            if let Some(dir) = resolver.get(dir_id) {
                if let Some(coords) = dir.get(0).and_then(|v| v.as_list()) {
                    return Vector3::new(
                        coords.first().and_then(|v| v.as_float()).unwrap_or(0.0),
                        coords.get(1).and_then(|v| v.as_float()).unwrap_or(1.0),
                        coords.get(2).and_then(|v| v.as_float()).unwrap_or(0.0),
                    ).normalize();
                }
            }
        }
        Vector3::new(0.0, 1.0, 0.0) // Default Y axis
    }
}

impl Default for RevolvedAreaSolidProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryProcessor for RevolvedAreaSolidProcessor {
    fn process(
        &self,
        entity: &DecodedEntity,
        resolver: &dyn EntityResolver,
        _unit_scale: f64,
    ) -> Result<Mesh> {
        // IfcRevolvedAreaSolid attributes:
        // 0: SweptArea (IfcProfileDef)
        // 1: Position (IfcAxis2Placement3D)
        // 2: Axis (IfcAxis1Placement)
        // 3: Angle

        let profile_id = entity
            .get_ref(0)
            .ok_or_else(|| Error::invalid_attribute(0, "Missing SweptArea"))?;

        let profile = resolver
            .get(profile_id)
            .ok_or_else(|| Error::entity_not_found(profile_id.0))?;

        let axis_id = entity
            .get_ref(2)
            .ok_or_else(|| Error::invalid_attribute(2, "Missing Axis"))?;

        let axis = resolver
            .get(axis_id)
            .ok_or_else(|| Error::entity_not_found(axis_id.0))?;

        let angle = entity
            .get_float(3)
            .ok_or_else(|| Error::invalid_attribute(3, "Missing Angle"))?;

        let profile_points = self.extract_profile(&profile, resolver)?;
        if profile_points.is_empty() {
            return Ok(Mesh::new());
        }

        let axis_location = self.parse_axis_location(&axis, resolver)?;
        let axis_direction = self.parse_axis_direction(&axis, resolver);

        // Generate revolved mesh
        let full_circle = angle.abs() >= std::f64::consts::PI * 1.99;
        let segments = if full_circle {
            24
        } else {
            ((angle.abs() / std::f64::consts::PI * 12.0).ceil() as usize).max(4)
        };

        let num_profile_points = profile_points.len();
        let mut positions = Vec::new();
        let mut indices = Vec::new();

        let (ax, ay, az) = (axis_direction.x, axis_direction.y, axis_direction.z);

        for i in 0..=segments {
            let t = if full_circle && i == segments {
                0.0
            } else {
                angle * i as f64 / segments as f64
            };

            let cos_t = t.cos();
            let sin_t = t.sin();

            // Rodrigues' rotation formula helper
            let k_matrix = |v: Vector3<f64>| -> Vector3<f64> {
                Vector3::new(
                    ay * v.z - az * v.y,
                    az * v.x - ax * v.z,
                    ax * v.y - ay * v.x,
                )
            };

            for (j, p2d) in profile_points.iter().enumerate() {
                let radius = p2d.x;
                let height = p2d.y;

                let v = Vector3::new(radius, 0.0, 0.0);

                let k_cross_v = k_matrix(v);
                let k_dot_v = ax * v.x + ay * v.y + az * v.z;

                let v_rot = v * cos_t + k_cross_v * sin_t + axis_direction * k_dot_v * (1.0 - cos_t);

                let pos = axis_location + axis_direction * height + v_rot;

                positions.push(pos.x as f32);
                positions.push(pos.y as f32);
                positions.push(pos.z as f32);

                if i < segments && j < num_profile_points - 1 {
                    let current = (i * num_profile_points + j) as u32;
                    let next_seg = ((i + 1) * num_profile_points + j) as u32;
                    let current_next = current + 1;
                    let next_seg_next = next_seg + 1;

                    indices.push(current);
                    indices.push(next_seg);
                    indices.push(next_seg_next);

                    indices.push(current);
                    indices.push(next_seg_next);
                    indices.push(current_next);
                }
            }
        }

        Ok(Mesh {
            positions,
            normals: Vec::new(),
            indices,
        })
    }

    fn supported_types(&self) -> Vec<IfcType> {
        vec![IfcType::IfcRevolvedAreaSolid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point3;

    #[test]
    fn test_extruded_area_solid_processor_creation() {
        let processor = ExtrudedAreaSolidProcessor::new();
        assert_eq!(
            processor.supported_types(),
            vec![IfcType::IfcExtrudedAreaSolid]
        );
    }

    #[test]
    fn test_triangulated_face_set_processor_creation() {
        let processor = TriangulatedFaceSetProcessor::new();
        assert_eq!(
            processor.supported_types(),
            vec![IfcType::IfcTriangulatedFaceSet]
        );
    }

    #[test]
    fn test_faceted_brep_processor_creation() {
        let processor = FacetedBrepProcessor::new();
        assert_eq!(
            processor.supported_types(),
            vec![IfcType::IfcFacetedBrep]
        );
    }

    #[test]
    fn test_swept_disk_solid_processor_creation() {
        let processor = SweptDiskSolidProcessor::new();
        assert_eq!(
            processor.supported_types(),
            vec![IfcType::IfcSweptDiskSolid]
        );
    }

    #[test]
    fn test_revolved_area_solid_processor_creation() {
        let processor = RevolvedAreaSolidProcessor::new();
        assert_eq!(
            processor.supported_types(),
            vec![IfcType::IfcRevolvedAreaSolid]
        );
    }

    #[test]
    fn test_faceted_brep_triangulate_triangle() {
        let processor = FacetedBrepProcessor::new();
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ];
        let (positions, indices) = processor.triangulate_face(&points, &[]);

        assert_eq!(positions.len(), 9); // 3 vertices * 3 components
        assert_eq!(indices.len(), 3);   // 1 triangle * 3 indices
    }

    #[test]
    fn test_faceted_brep_triangulate_quad() {
        let processor = FacetedBrepProcessor::new();
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let (positions, indices) = processor.triangulate_face(&points, &[]);

        assert_eq!(positions.len(), 12); // 4 vertices * 3 components
        assert_eq!(indices.len(), 6);    // 2 triangles * 3 indices
    }

    #[test]
    fn test_faceted_brep_triangulate_with_hole() {
        let processor = FacetedBrepProcessor::new();
        let outer = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
            Point3::new(4.0, 4.0, 0.0),
            Point3::new(0.0, 4.0, 0.0),
        ];
        let hole = vec![
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(3.0, 1.0, 0.0),
            Point3::new(3.0, 3.0, 0.0),
            Point3::new(1.0, 3.0, 0.0),
        ];
        let (positions, indices) = processor.triangulate_face(&outer, &[hole]);

        // Should have 8 vertices (4 outer + 4 hole)
        assert_eq!(positions.len(), 24);
        // Should have multiple triangles to fill the ring
        assert!(indices.len() >= 18); // At least 6 triangles
    }
}
