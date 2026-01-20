// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Core types for IFC data representation
//!
//! This module defines the fundamental types used throughout the IFC parsing system.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Type-safe entity identifier
///
/// Wraps the raw IFC entity ID (e.g., #123 becomes EntityId(123))
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub struct EntityId(pub u32);

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl From<u32> for EntityId {
    fn from(id: u32) -> Self {
        EntityId(id)
    }
}

impl From<EntityId> for u32 {
    fn from(id: EntityId) -> Self {
        id.0
    }
}

impl From<EntityId> for u64 {
    fn from(id: EntityId) -> Self {
        id.0 as u64
    }
}

/// IFC entity type enumeration
///
/// Covers all common IFC entity types. Unknown types are captured with their
/// original string representation.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IfcType {
    // ========================================================================
    // Spatial Structure
    // ========================================================================
    IfcProject,
    IfcSite,
    IfcBuilding,
    IfcBuildingStorey,
    IfcSpace,

    // ========================================================================
    // Building Elements
    // ========================================================================
    IfcWall,
    IfcWallStandardCase,
    IfcCurtainWall,
    IfcSlab,
    IfcRoof,
    IfcBeam,
    IfcColumn,
    IfcDoor,
    IfcWindow,
    IfcStair,
    IfcStairFlight,
    IfcRamp,
    IfcRampFlight,
    IfcRailing,
    IfcCovering,
    IfcPlate,
    IfcMember,
    IfcFooting,
    IfcPile,
    IfcBuildingElementProxy,

    // ========================================================================
    // Distribution Elements (MEP)
    // ========================================================================
    IfcDistributionElement,
    IfcDistributionFlowElement,
    IfcFlowTerminal,
    IfcFlowSegment,
    IfcFlowFitting,
    IfcFlowController,
    IfcFlowMovingDevice,
    IfcFlowStorageDevice,
    IfcFlowTreatmentDevice,
    IfcEnergyConversionDevice,
    IfcDistributionControlElement,

    // ========================================================================
    // Furnishing and Equipment
    // ========================================================================
    IfcFurnishingElement,
    IfcFurniture,
    IfcSystemFurnitureElement,

    // ========================================================================
    // Openings and Features
    // ========================================================================
    IfcOpeningElement,
    IfcOpeningStandardCase,
    IfcVoidingFeature,
    IfcProjectionElement,

    // ========================================================================
    // Geometry Representations
    // ========================================================================
    // Swept solids
    IfcExtrudedAreaSolid,
    IfcExtrudedAreaSolidTapered,
    IfcRevolvedAreaSolid,
    IfcRevolvedAreaSolidTapered,
    IfcSweptDiskSolid,
    IfcSweptDiskSolidPolygonal,
    IfcSurfaceCurveSweptAreaSolid,
    IfcFixedReferenceSweptAreaSolid,

    // Boundary representations
    IfcFacetedBrep,
    IfcFacetedBrepWithVoids,
    IfcAdvancedBrep,
    IfcAdvancedBrepWithVoids,

    // Tessellated geometry (IFC4+)
    IfcTriangulatedFaceSet,
    IfcPolygonalFaceSet,
    IfcTessellatedFaceSet,

    // Boolean operations
    IfcBooleanResult,
    IfcBooleanClippingResult,

    // Mapped items (instancing)
    IfcMappedItem,
    IfcRepresentationMap,

    // CSG primitives
    IfcBlock,
    IfcRectangularPyramid,
    IfcRightCircularCone,
    IfcRightCircularCylinder,
    IfcSphere,

    // Half-space solids
    IfcHalfSpaceSolid,
    IfcPolygonalBoundedHalfSpace,
    IfcBoxedHalfSpace,

    // ========================================================================
    // Profiles (2D cross-sections)
    // ========================================================================
    IfcArbitraryClosedProfileDef,
    IfcArbitraryProfileDefWithVoids,
    IfcRectangleProfileDef,
    IfcRectangleHollowProfileDef,
    IfcCircleProfileDef,
    IfcCircleHollowProfileDef,
    IfcEllipseProfileDef,
    IfcIShapeProfileDef,
    IfcLShapeProfileDef,
    IfcTShapeProfileDef,
    IfcUShapeProfileDef,
    IfcCShapeProfileDef,
    IfcZShapeProfileDef,
    IfcAsymmetricIShapeProfileDef,
    IfcTrapeziumProfileDef,
    IfcCompositeProfileDef,
    IfcDerivedProfileDef,
    IfcCenterLineProfileDef,

    // ========================================================================
    // Curves
    // ========================================================================
    IfcPolyline,
    IfcCompositeCurve,
    IfcCompositeCurveSegment,
    IfcTrimmedCurve,
    IfcCircle,
    IfcEllipse,
    IfcLine,
    IfcBSplineCurve,
    IfcBSplineCurveWithKnots,
    IfcRationalBSplineCurveWithKnots,
    IfcIndexedPolyCurve,

    // ========================================================================
    // Surfaces
    // ========================================================================
    IfcPlane,
    IfcCurveBoundedPlane,
    IfcCylindricalSurface,
    IfcBSplineSurface,
    IfcBSplineSurfaceWithKnots,
    IfcRationalBSplineSurfaceWithKnots,

    // ========================================================================
    // Points and Directions
    // ========================================================================
    IfcCartesianPoint,
    IfcDirection,
    IfcVector,
    IfcCartesianPointList2D,
    IfcCartesianPointList3D,

    // ========================================================================
    // Placement and Transforms
    // ========================================================================
    IfcAxis2Placement2D,
    IfcAxis2Placement3D,
    IfcLocalPlacement,
    IfcCartesianTransformationOperator3D,
    IfcCartesianTransformationOperator3DnonUniform,

    // ========================================================================
    // Representations and Contexts
    // ========================================================================
    IfcShapeRepresentation,
    IfcProductDefinitionShape,
    IfcGeometricRepresentationContext,
    IfcGeometricRepresentationSubContext,

    // ========================================================================
    // Topology
    // ========================================================================
    IfcClosedShell,
    IfcOpenShell,
    IfcFace,
    IfcFaceBound,
    IfcFaceOuterBound,
    IfcPolyLoop,
    IfcEdgeLoop,
    IfcOrientedEdge,
    IfcEdgeCurve,
    IfcVertexPoint,
    IfcConnectedFaceSet,

    // ========================================================================
    // Relationships
    // ========================================================================
    IfcRelContainedInSpatialStructure,
    IfcRelAggregates,
    IfcRelDefinesByProperties,
    IfcRelDefinesByType,
    IfcRelAssociatesMaterial,
    IfcRelVoidsElement,
    IfcRelFillsElement,
    IfcRelConnectsPathElements,
    IfcRelSpaceBoundary,

    // ========================================================================
    // Properties
    // ========================================================================
    IfcPropertySet,
    IfcPropertySingleValue,
    IfcPropertyEnumeratedValue,
    IfcPropertyBoundedValue,
    IfcPropertyListValue,
    IfcPropertyTableValue,
    IfcComplexProperty,
    IfcElementQuantity,
    IfcQuantityLength,
    IfcQuantityArea,
    IfcQuantityVolume,
    IfcQuantityCount,
    IfcQuantityWeight,
    IfcQuantityTime,

    // ========================================================================
    // Materials
    // ========================================================================
    IfcMaterial,
    IfcMaterialLayer,
    IfcMaterialLayerSet,
    IfcMaterialLayerSetUsage,
    IfcMaterialList,
    IfcMaterialConstituentSet,
    IfcMaterialConstituent,
    IfcMaterialProfile,
    IfcMaterialProfileSet,
    IfcMaterialProfileSetUsage,

    // ========================================================================
    // Presentation (styling)
    // ========================================================================
    IfcStyledItem,
    IfcSurfaceStyle,
    IfcSurfaceStyleRendering,
    IfcColourRgb,
    IfcPresentationLayerAssignment,

    // ========================================================================
    // Units
    // ========================================================================
    IfcUnitAssignment,
    IfcSIUnit,
    IfcConversionBasedUnit,
    IfcDerivedUnit,
    IfcMeasureWithUnit,

    // ========================================================================
    // Type definitions
    // ========================================================================
    IfcWallType,
    IfcSlabType,
    IfcBeamType,
    IfcColumnType,
    IfcDoorType,
    IfcWindowType,
    IfcCoveringType,
    IfcRailingType,
    IfcStairType,
    IfcStairFlightType,
    IfcRampType,
    IfcRampFlightType,
    IfcRoofType,
    IfcMemberType,
    IfcPlateType,
    IfcFootingType,
    IfcPileType,
    IfcBuildingElementProxyType,

    // ========================================================================
    // IFC4x3 Additions (Infrastructure)
    // ========================================================================
    IfcAlignment,
    IfcAlignmentCant,
    IfcAlignmentHorizontal,
    IfcAlignmentVertical,
    IfcAlignmentSegment,
    IfcRoad,
    IfcRoadPart,
    IfcBridge,
    IfcBridgePart,
    IfcRailway,
    IfcRailwayPart,
    IfcFacility,
    IfcFacilityPart,
    IfcGeotechnicalElement,
    IfcBorehole,
    IfcGeomodel,
    IfcGeoslice,
    IfcSolidStratum,
    IfcVoidStratum,
    IfcWaterStratum,
    IfcEarthworksCut,
    IfcEarthworksFill,
    IfcEarthworksElement,
    IfcPavement,
    IfcCourse,
    IfcKerb,
    IfcDeepFoundation,

    /// Unknown type - stores the original type name string
    Unknown(String),
}

impl FromStr for IfcType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::parse(s))
    }
}

impl IfcType {
    /// Parse a type name string into an IfcType
    pub fn parse(s: &str) -> Self {
        // Convert to uppercase for matching
        match s.to_uppercase().as_str() {
            // Spatial structure
            "IFCPROJECT" => IfcType::IfcProject,
            "IFCSITE" => IfcType::IfcSite,
            "IFCBUILDING" => IfcType::IfcBuilding,
            "IFCBUILDINGSTOREY" => IfcType::IfcBuildingStorey,
            "IFCSPACE" => IfcType::IfcSpace,

            // Building elements
            "IFCWALL" => IfcType::IfcWall,
            "IFCWALLSTANDARDCASE" => IfcType::IfcWallStandardCase,
            "IFCCURTAINWALL" => IfcType::IfcCurtainWall,
            "IFCSLAB" => IfcType::IfcSlab,
            "IFCROOF" => IfcType::IfcRoof,
            "IFCBEAM" => IfcType::IfcBeam,
            "IFCCOLUMN" => IfcType::IfcColumn,
            "IFCDOOR" => IfcType::IfcDoor,
            "IFCWINDOW" => IfcType::IfcWindow,
            "IFCSTAIR" => IfcType::IfcStair,
            "IFCSTAIRFLIGHT" => IfcType::IfcStairFlight,
            "IFCRAMP" => IfcType::IfcRamp,
            "IFCRAMPFLIGHT" => IfcType::IfcRampFlight,
            "IFCRAILING" => IfcType::IfcRailing,
            "IFCCOVERING" => IfcType::IfcCovering,
            "IFCPLATE" => IfcType::IfcPlate,
            "IFCMEMBER" => IfcType::IfcMember,
            "IFCFOOTING" => IfcType::IfcFooting,
            "IFCPILE" => IfcType::IfcPile,
            "IFCBUILDINGELEMENTPROXY" => IfcType::IfcBuildingElementProxy,

            // Distribution elements
            "IFCDISTRIBUTIONELEMENT" => IfcType::IfcDistributionElement,
            "IFCDISTRIBUTIONFLOWELEMENT" => IfcType::IfcDistributionFlowElement,
            "IFCFLOWTERMINAL" => IfcType::IfcFlowTerminal,
            "IFCFLOWSEGMENT" => IfcType::IfcFlowSegment,
            "IFCFLOWFITTING" => IfcType::IfcFlowFitting,
            "IFCFLOWCONTROLLER" => IfcType::IfcFlowController,
            "IFCFLOWMOVINGDEVICE" => IfcType::IfcFlowMovingDevice,
            "IFCFLOWSTORAGEDEVICE" => IfcType::IfcFlowStorageDevice,
            "IFCFLOWTREATMENTDEVICE" => IfcType::IfcFlowTreatmentDevice,
            "IFCENERGYCONVERSIONDEVICE" => IfcType::IfcEnergyConversionDevice,
            "IFCDISTRIBUTIONCONTROLELEMENT" => IfcType::IfcDistributionControlElement,

            // Furnishing
            "IFCFURNISHINGELEMENT" => IfcType::IfcFurnishingElement,
            "IFCFURNITURE" => IfcType::IfcFurniture,
            "IFCSYSTEMFURNITUREELEMENT" => IfcType::IfcSystemFurnitureElement,

            // Openings
            "IFCOPENINGELEMENT" => IfcType::IfcOpeningElement,
            "IFCOPENINGSTANDARDCASE" => IfcType::IfcOpeningStandardCase,
            "IFCVOIDINGFEATURE" => IfcType::IfcVoidingFeature,
            "IFCPROJECTIONELEMENT" => IfcType::IfcProjectionElement,

            // Geometry - Swept solids
            "IFCEXTRUDEDAREASOLID" => IfcType::IfcExtrudedAreaSolid,
            "IFCEXTRUDEDAREASOLIDTAPERED" => IfcType::IfcExtrudedAreaSolidTapered,
            "IFCREVOLVEDAREASOLID" => IfcType::IfcRevolvedAreaSolid,
            "IFCREVOLVEDAREASOLIDTAPERED" => IfcType::IfcRevolvedAreaSolidTapered,
            "IFCSWEPTDISKSOLID" => IfcType::IfcSweptDiskSolid,
            "IFCSWEPTDISKSOLIDPOLYGONAL" => IfcType::IfcSweptDiskSolidPolygonal,
            "IFCSURFACECURVESWEPTAREASOLID" => IfcType::IfcSurfaceCurveSweptAreaSolid,
            "IFCFIXEDREFERENCESWEPTAREASOLID" => IfcType::IfcFixedReferenceSweptAreaSolid,

            // Geometry - BREPs
            "IFCFACETEDBREP" => IfcType::IfcFacetedBrep,
            "IFCFACETEDBREPWITHVOIDS" => IfcType::IfcFacetedBrepWithVoids,
            "IFCADVANCEDBREP" => IfcType::IfcAdvancedBrep,
            "IFCADVANCEDBREPWITHVOIDS" => IfcType::IfcAdvancedBrepWithVoids,

            // Geometry - Tessellated
            "IFCTRIANGULATEDFACESET" => IfcType::IfcTriangulatedFaceSet,
            "IFCPOLYGONALFACESET" => IfcType::IfcPolygonalFaceSet,
            "IFCTESSELLATEDFACESET" => IfcType::IfcTessellatedFaceSet,

            // Geometry - Boolean
            "IFCBOOLEANRESULT" => IfcType::IfcBooleanResult,
            "IFCBOOLEANCLIPPINGRESULT" => IfcType::IfcBooleanClippingResult,

            // Geometry - Mapped items
            "IFCMAPPEDITEM" => IfcType::IfcMappedItem,
            "IFCREPRESENTATIONMAP" => IfcType::IfcRepresentationMap,

            // Geometry - CSG primitives
            "IFCBLOCK" => IfcType::IfcBlock,
            "IFCRECTANGULARPYRAMID" => IfcType::IfcRectangularPyramid,
            "IFCRIGHTCIRCULARCONE" => IfcType::IfcRightCircularCone,
            "IFCRIGHTCIRCULARCYLINDER" => IfcType::IfcRightCircularCylinder,
            "IFCSPHERE" => IfcType::IfcSphere,

            // Geometry - Half-space
            "IFCHALFSPACESOLID" => IfcType::IfcHalfSpaceSolid,
            "IFCPOLYGONALBOUNDEDHALFSPACE" => IfcType::IfcPolygonalBoundedHalfSpace,
            "IFCBOXEDHALFSPACE" => IfcType::IfcBoxedHalfSpace,

            // Profiles
            "IFCARBITRARYCLOSEDPROFILEDEF" => IfcType::IfcArbitraryClosedProfileDef,
            "IFCARBITRARYPROFILEDEFWITHVOIDS" => IfcType::IfcArbitraryProfileDefWithVoids,
            "IFCRECTANGLEPROFILEDEF" => IfcType::IfcRectangleProfileDef,
            "IFCRECTANGLEHOLLOWPROFILEDEF" => IfcType::IfcRectangleHollowProfileDef,
            "IFCCIRCLEPROFILEDEF" => IfcType::IfcCircleProfileDef,
            "IFCCIRCLEHOLLOWPROFILEDEF" => IfcType::IfcCircleHollowProfileDef,
            "IFCELLIPSEPROFILEDEF" => IfcType::IfcEllipseProfileDef,
            "IFCISHAPEPROFILEDEF" => IfcType::IfcIShapeProfileDef,
            "IFCLSHAPEPROFILEDEF" => IfcType::IfcLShapeProfileDef,
            "IFCTSHAPEPROFILEDEF" => IfcType::IfcTShapeProfileDef,
            "IFCUSHAPEPROFILEDEF" => IfcType::IfcUShapeProfileDef,
            "IFCCSHAPEPROFILEDEF" => IfcType::IfcCShapeProfileDef,
            "IFCZSHAPEPROFILEDEF" => IfcType::IfcZShapeProfileDef,
            "IFCASYMMETRICISHAPEPROFILEDEF" => IfcType::IfcAsymmetricIShapeProfileDef,
            "IFCTRAPEZIUMPROFILEDEF" => IfcType::IfcTrapeziumProfileDef,
            "IFCCOMPOSITEPROFILEDEF" => IfcType::IfcCompositeProfileDef,
            "IFCDERIVEDPROFILEDEF" => IfcType::IfcDerivedProfileDef,
            "IFCCENTERLINEPROFILEDEF" => IfcType::IfcCenterLineProfileDef,

            // Curves
            "IFCPOLYLINE" => IfcType::IfcPolyline,
            "IFCCOMPOSITECURVE" => IfcType::IfcCompositeCurve,
            "IFCCOMPOSITECURVESEGMENT" => IfcType::IfcCompositeCurveSegment,
            "IFCTRIMMEDCURVE" => IfcType::IfcTrimmedCurve,
            "IFCCIRCLE" => IfcType::IfcCircle,
            "IFCELLIPSE" => IfcType::IfcEllipse,
            "IFCLINE" => IfcType::IfcLine,
            "IFCBSPLINECURVE" => IfcType::IfcBSplineCurve,
            "IFCBSPLINECURVEWITHKNOTS" => IfcType::IfcBSplineCurveWithKnots,
            "IFCRATIONALBSPLINECURVEWITHKNOTS" => IfcType::IfcRationalBSplineCurveWithKnots,
            "IFCINDEXEDPOLYCURVE" => IfcType::IfcIndexedPolyCurve,

            // Surfaces
            "IFCPLANE" => IfcType::IfcPlane,
            "IFCCURVEBOUNDEDPLANE" => IfcType::IfcCurveBoundedPlane,
            "IFCCYLINDRICALSURFACE" => IfcType::IfcCylindricalSurface,
            "IFCBSPLINESURFACE" => IfcType::IfcBSplineSurface,
            "IFCBSPLINESURFACEWITHKNOTS" => IfcType::IfcBSplineSurfaceWithKnots,
            "IFCRATIONALBSPLINESURFACEWITHKNOTS" => IfcType::IfcRationalBSplineSurfaceWithKnots,

            // Points and directions
            "IFCCARTESIANPOINT" => IfcType::IfcCartesianPoint,
            "IFCDIRECTION" => IfcType::IfcDirection,
            "IFCVECTOR" => IfcType::IfcVector,
            "IFCCARTESIANPOINTLIST2D" => IfcType::IfcCartesianPointList2D,
            "IFCCARTESIANPOINTLIST3D" => IfcType::IfcCartesianPointList3D,

            // Placement
            "IFCAXIS2PLACEMENT2D" => IfcType::IfcAxis2Placement2D,
            "IFCAXIS2PLACEMENT3D" => IfcType::IfcAxis2Placement3D,
            "IFCLOCALPLACEMENT" => IfcType::IfcLocalPlacement,
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D" => IfcType::IfcCartesianTransformationOperator3D,
            "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM" => {
                IfcType::IfcCartesianTransformationOperator3DnonUniform
            }

            // Representations
            "IFCSHAPEREPRESENTATION" => IfcType::IfcShapeRepresentation,
            "IFCPRODUCTDEFINITIONSHAPE" => IfcType::IfcProductDefinitionShape,
            "IFCGEOMETRICREPRESENTATIONCONTEXT" => IfcType::IfcGeometricRepresentationContext,
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT" => IfcType::IfcGeometricRepresentationSubContext,

            // Topology
            "IFCCLOSEDSHELL" => IfcType::IfcClosedShell,
            "IFCOPENSHELL" => IfcType::IfcOpenShell,
            "IFCFACE" => IfcType::IfcFace,
            "IFCFACEBOUND" => IfcType::IfcFaceBound,
            "IFCFACEOUTERBOUND" => IfcType::IfcFaceOuterBound,
            "IFCPOLYLOOP" => IfcType::IfcPolyLoop,
            "IFCEDGELOOP" => IfcType::IfcEdgeLoop,
            "IFCORIENTEDEDGE" => IfcType::IfcOrientedEdge,
            "IFCEDGECURVE" => IfcType::IfcEdgeCurve,
            "IFCVERTEXPOINT" => IfcType::IfcVertexPoint,
            "IFCCONNECTEDFACESET" => IfcType::IfcConnectedFaceSet,

            // Relationships
            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => IfcType::IfcRelContainedInSpatialStructure,
            "IFCRELAGGREGATES" => IfcType::IfcRelAggregates,
            "IFCRELDEFINESBYPROPERTIES" => IfcType::IfcRelDefinesByProperties,
            "IFCRELDEFINESBYTYPE" => IfcType::IfcRelDefinesByType,
            "IFCRELASSOCIATESMATERIAL" => IfcType::IfcRelAssociatesMaterial,
            "IFCRELVOIDSELEMENT" => IfcType::IfcRelVoidsElement,
            "IFCRELFILLSELEMENT" => IfcType::IfcRelFillsElement,
            "IFCRELCONNECTSPATHELEMENTS" => IfcType::IfcRelConnectsPathElements,
            "IFCRELSPACEBOUNDARY" => IfcType::IfcRelSpaceBoundary,

            // Properties
            "IFCPROPERTYSET" => IfcType::IfcPropertySet,
            "IFCPROPERTYSINGLEVALUE" => IfcType::IfcPropertySingleValue,
            "IFCPROPERTYENUMERATEDVALUE" => IfcType::IfcPropertyEnumeratedValue,
            "IFCPROPERTYBOUNDEDVALUE" => IfcType::IfcPropertyBoundedValue,
            "IFCPROPERTYLISTVALUE" => IfcType::IfcPropertyListValue,
            "IFCPROPERTYTABLEVALUE" => IfcType::IfcPropertyTableValue,
            "IFCCOMPLEXPROPERTY" => IfcType::IfcComplexProperty,
            "IFCELEMENTQUANTITY" => IfcType::IfcElementQuantity,
            "IFCQUANTITYLENGTH" => IfcType::IfcQuantityLength,
            "IFCQUANTITYAREA" => IfcType::IfcQuantityArea,
            "IFCQUANTITYVOLUME" => IfcType::IfcQuantityVolume,
            "IFCQUANTITYCOUNT" => IfcType::IfcQuantityCount,
            "IFCQUANTITYWEIGHT" => IfcType::IfcQuantityWeight,
            "IFCQUANTITYTIME" => IfcType::IfcQuantityTime,

            // Materials
            "IFCMATERIAL" => IfcType::IfcMaterial,
            "IFCMATERIALLAYER" => IfcType::IfcMaterialLayer,
            "IFCMATERIALLAYERSET" => IfcType::IfcMaterialLayerSet,
            "IFCMATERIALLAYERSETUSAGE" => IfcType::IfcMaterialLayerSetUsage,
            "IFCMATERIALLIST" => IfcType::IfcMaterialList,
            "IFCMATERIALCONSTITUENTSET" => IfcType::IfcMaterialConstituentSet,
            "IFCMATERIALCONSTITUENT" => IfcType::IfcMaterialConstituent,
            "IFCMATERIALPROFILE" => IfcType::IfcMaterialProfile,
            "IFCMATERIALPROFILESET" => IfcType::IfcMaterialProfileSet,
            "IFCMATERIALPROFILESETUSAGE" => IfcType::IfcMaterialProfileSetUsage,

            // Presentation
            "IFCSTYLEDITEM" => IfcType::IfcStyledItem,
            "IFCSURFACESTYLE" => IfcType::IfcSurfaceStyle,
            "IFCSURFACESTYLERENDERING" => IfcType::IfcSurfaceStyleRendering,
            "IFCCOLOURRGB" => IfcType::IfcColourRgb,
            "IFCPRESENTATIONLAYERASSIGNMENT" => IfcType::IfcPresentationLayerAssignment,

            // Units
            "IFCUNITASSIGNMENT" => IfcType::IfcUnitAssignment,
            "IFCSIUNIT" => IfcType::IfcSIUnit,
            "IFCCONVERSIONBASEDUNIT" => IfcType::IfcConversionBasedUnit,
            "IFCDERIVEDUNIT" => IfcType::IfcDerivedUnit,
            "IFCMEASUREWITHUNIT" => IfcType::IfcMeasureWithUnit,

            // Type definitions
            "IFCWALLTYPE" => IfcType::IfcWallType,
            "IFCSLABTYPE" => IfcType::IfcSlabType,
            "IFCBEAMTYPE" => IfcType::IfcBeamType,
            "IFCCOLUMNTYPE" => IfcType::IfcColumnType,
            "IFCDOORTYPE" => IfcType::IfcDoorType,
            "IFCWINDOWTYPE" => IfcType::IfcWindowType,
            "IFCCOVERINGTYPE" => IfcType::IfcCoveringType,
            "IFCRAILINGTYPE" => IfcType::IfcRailingType,
            "IFCSTAIRTYPE" => IfcType::IfcStairType,
            "IFCSTAIRFLIGHTTYPE" => IfcType::IfcStairFlightType,
            "IFCRAMPTYPE" => IfcType::IfcRampType,
            "IFCRAMPFLIGHTTYPE" => IfcType::IfcRampFlightType,
            "IFCROOFTYPE" => IfcType::IfcRoofType,
            "IFCMEMBERTYPE" => IfcType::IfcMemberType,
            "IFCPLATETYPE" => IfcType::IfcPlateType,
            "IFCFOOTINGTYPE" => IfcType::IfcFootingType,
            "IFCPILETYPE" => IfcType::IfcPileType,
            "IFCBUILDINGELEMENTPROXYTYPE" => IfcType::IfcBuildingElementProxyType,

            // IFC4x3 Infrastructure
            "IFCALIGNMENT" => IfcType::IfcAlignment,
            "IFCALIGNMENTCANT" => IfcType::IfcAlignmentCant,
            "IFCALIGNMENTHORIZONTAL" => IfcType::IfcAlignmentHorizontal,
            "IFCALIGNMENTVERTICAL" => IfcType::IfcAlignmentVertical,
            "IFCALIGNMENTSEGMENT" => IfcType::IfcAlignmentSegment,
            "IFCROAD" => IfcType::IfcRoad,
            "IFCROADPART" => IfcType::IfcRoadPart,
            "IFCBRIDGE" => IfcType::IfcBridge,
            "IFCBRIDGEPART" => IfcType::IfcBridgePart,
            "IFCRAILWAY" => IfcType::IfcRailway,
            "IFCRAILWAYPART" => IfcType::IfcRailwayPart,
            "IFCFACILITY" => IfcType::IfcFacility,
            "IFCFACILITYPART" => IfcType::IfcFacilityPart,
            "IFCGEOTECHNICALELEMENT" => IfcType::IfcGeotechnicalElement,
            "IFCBOREHOLE" => IfcType::IfcBorehole,
            "IFCGEOMODEL" => IfcType::IfcGeomodel,
            "IFCGEOSLICE" => IfcType::IfcGeoslice,
            "IFCSOLIDSTRATUM" => IfcType::IfcSolidStratum,
            "IFCVOIDSTRATUM" => IfcType::IfcVoidStratum,
            "IFCWATERSTRATUM" => IfcType::IfcWaterStratum,
            "IFCEARTHWORKSCUT" => IfcType::IfcEarthworksCut,
            "IFCEARTHWORKSFILL" => IfcType::IfcEarthworksFill,
            "IFCEARTHWORKSELEMENT" => IfcType::IfcEarthworksElement,
            "IFCPAVEMENT" => IfcType::IfcPavement,
            "IFCCOURSE" => IfcType::IfcCourse,
            "IFCKERB" => IfcType::IfcKerb,
            "IFCDEEPFOUNDATION" => IfcType::IfcDeepFoundation,

            // Unknown
            _ => IfcType::Unknown(s.to_string()),
        }
    }

    /// Get the type name as a string
    pub fn name(&self) -> &str {
        match self {
            IfcType::Unknown(s) => s,
            _ => {
                // For known types, return the variant name
                // This uses the debug representation which includes the type name
                // A more elegant solution would use a macro, but this works
                match self {
                    IfcType::IfcProject => "IFCPROJECT",
                    IfcType::IfcSite => "IFCSITE",
                    IfcType::IfcBuilding => "IFCBUILDING",
                    IfcType::IfcBuildingStorey => "IFCBUILDINGSTOREY",
                    IfcType::IfcSpace => "IFCSPACE",
                    IfcType::IfcWall => "IFCWALL",
                    IfcType::IfcWallStandardCase => "IFCWALLSTANDARDCASE",
                    IfcType::IfcCurtainWall => "IFCCURTAINWALL",
                    IfcType::IfcSlab => "IFCSLAB",
                    IfcType::IfcRoof => "IFCROOF",
                    IfcType::IfcBeam => "IFCBEAM",
                    IfcType::IfcColumn => "IFCCOLUMN",
                    IfcType::IfcDoor => "IFCDOOR",
                    IfcType::IfcWindow => "IFCWINDOW",
                    IfcType::IfcStair => "IFCSTAIR",
                    IfcType::IfcStairFlight => "IFCSTAIRFLIGHT",
                    IfcType::IfcRamp => "IFCRAMP",
                    IfcType::IfcRampFlight => "IFCRAMPFLIGHT",
                    IfcType::IfcRailing => "IFCRAILING",
                    IfcType::IfcCovering => "IFCCOVERING",
                    IfcType::IfcPlate => "IFCPLATE",
                    IfcType::IfcMember => "IFCMEMBER",
                    IfcType::IfcFooting => "IFCFOOTING",
                    IfcType::IfcPile => "IFCPILE",
                    IfcType::IfcBuildingElementProxy => "IFCBUILDINGELEMENTPROXY",
                    IfcType::IfcExtrudedAreaSolid => "IFCEXTRUDEDAREASOLID",
                    IfcType::IfcFacetedBrep => "IFCFACETEDBREP",
                    IfcType::IfcTriangulatedFaceSet => "IFCTRIANGULATEDFACESET",
                    IfcType::IfcMappedItem => "IFCMAPPEDITEM",
                    IfcType::IfcBooleanClippingResult => "IFCBOOLEANCLIPPINGRESULT",
                    // Add more as needed, or use a macro for all variants
                    _ => "UNKNOWN",
                }
            }
        }
    }

    /// Check if this type represents a building element with potential geometry
    pub fn has_geometry(&self) -> bool {
        matches!(
            self,
            IfcType::IfcWall
                | IfcType::IfcWallStandardCase
                | IfcType::IfcCurtainWall
                | IfcType::IfcSlab
                | IfcType::IfcRoof
                | IfcType::IfcBeam
                | IfcType::IfcColumn
                | IfcType::IfcDoor
                | IfcType::IfcWindow
                | IfcType::IfcStair
                | IfcType::IfcStairFlight
                | IfcType::IfcRamp
                | IfcType::IfcRampFlight
                | IfcType::IfcRailing
                | IfcType::IfcCovering
                | IfcType::IfcPlate
                | IfcType::IfcMember
                | IfcType::IfcFooting
                | IfcType::IfcPile
                | IfcType::IfcBuildingElementProxy
                | IfcType::IfcFurnishingElement
                | IfcType::IfcFurniture
                | IfcType::IfcDistributionElement
                | IfcType::IfcFlowTerminal
                | IfcType::IfcFlowSegment
                | IfcType::IfcFlowFitting
                | IfcType::IfcOpeningElement
        )
    }

    /// Check if this type is a spatial structure element
    pub fn is_spatial(&self) -> bool {
        matches!(
            self,
            IfcType::IfcProject
                | IfcType::IfcSite
                | IfcType::IfcBuilding
                | IfcType::IfcBuildingStorey
                | IfcType::IfcSpace
                | IfcType::IfcFacility
                | IfcType::IfcFacilityPart
        )
    }
}

impl Default for IfcType {
    fn default() -> Self {
        IfcType::Unknown(String::new())
    }
}

impl fmt::Display for IfcType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Decoded attribute value
///
/// Represents any value that can appear in an IFC entity's attribute list.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum AttributeValue {
    /// Null value ($)
    #[default]
    Null,
    /// Derived value (*)
    Derived,
    /// Entity reference (#123)
    EntityRef(EntityId),
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Enumeration value (.VALUE.)
    Enum(String),
    /// List of values
    List(Vec<AttributeValue>),
    /// Typed value like IFCLABEL('text')
    TypedValue(String, Vec<AttributeValue>),
}

impl AttributeValue {
    /// Try to get as entity reference
    pub fn as_entity_ref(&self) -> Option<EntityId> {
        match self {
            AttributeValue::EntityRef(id) => Some(*id),
            _ => None,
        }
    }

    /// Try to get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            AttributeValue::String(s) => Some(s),
            AttributeValue::TypedValue(_, args) if !args.is_empty() => args[0].as_string(),
            _ => None,
        }
    }

    /// Try to get as float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            AttributeValue::Float(f) => Some(*f),
            AttributeValue::Integer(i) => Some(*i as f64),
            AttributeValue::TypedValue(_, args) if !args.is_empty() => args[0].as_float(),
            _ => None,
        }
    }

    /// Try to get as integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            AttributeValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AttributeValue::Bool(b) => Some(*b),
            AttributeValue::Enum(s) => match s.to_uppercase().as_str() {
                "TRUE" | "T" => Some(true),
                "FALSE" | "F" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Try to get as enum string
    pub fn as_enum(&self) -> Option<&str> {
        match self {
            AttributeValue::Enum(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as list
    pub fn as_list(&self) -> Option<&[AttributeValue]> {
        match self {
            AttributeValue::List(list) => Some(list),
            _ => None,
        }
    }

    /// Check if this is a null value
    pub fn is_null(&self) -> bool {
        matches!(self, AttributeValue::Null)
    }

    /// Check if this is a derived value
    pub fn is_derived(&self) -> bool {
        matches!(self, AttributeValue::Derived)
    }
}

/// Decoded IFC entity
///
/// Represents a fully decoded IFC entity with its ID, type, and attribute values.
#[derive(Clone, Debug)]
pub struct DecodedEntity {
    /// Entity ID
    pub id: EntityId,
    /// Entity type
    pub ifc_type: IfcType,
    /// Attribute values in order
    pub attributes: Vec<AttributeValue>,
}

impl DecodedEntity {
    /// Get attribute at index
    pub fn get(&self, index: usize) -> Option<&AttributeValue> {
        self.attributes.get(index)
    }

    /// Get entity reference at index
    pub fn get_ref(&self, index: usize) -> Option<EntityId> {
        self.get(index).and_then(|v| v.as_entity_ref())
    }

    /// Get string at index
    pub fn get_string(&self, index: usize) -> Option<&str> {
        self.get(index).and_then(|v| v.as_string())
    }

    /// Get float at index
    pub fn get_float(&self, index: usize) -> Option<f64> {
        self.get(index).and_then(|v| v.as_float())
    }

    /// Get integer at index
    pub fn get_integer(&self, index: usize) -> Option<i64> {
        self.get(index).and_then(|v| v.as_integer())
    }

    /// Get list at index
    pub fn get_list(&self, index: usize) -> Option<&[AttributeValue]> {
        self.get(index).and_then(|v| v.as_list())
    }

    /// Get boolean at index
    pub fn get_bool(&self, index: usize) -> Option<bool> {
        self.get(index).and_then(|v| v.as_bool())
    }

    /// Get enum string at index
    pub fn get_enum(&self, index: usize) -> Option<&str> {
        self.get(index).and_then(|v| v.as_enum())
    }

    /// Get list of entity references at index
    pub fn get_refs(&self, index: usize) -> Option<Vec<EntityId>> {
        self.get_list(index)
            .map(|list| list.iter().filter_map(|v| v.as_entity_ref()).collect())
    }
}

/// GPU-ready mesh data
///
/// Contains flattened vertex data suitable for GPU rendering.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    /// Vertex positions as flattened [x, y, z, x, y, z, ...]
    pub positions: Vec<f32>,
    /// Vertex normals as flattened [nx, ny, nz, nx, ny, nz, ...]
    pub normals: Vec<f32>,
    /// Triangle indices
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self::default()
    }

    /// Create mesh with pre-allocated capacity
    pub fn with_capacity(vertex_count: usize, index_count: usize) -> Self {
        Self {
            positions: Vec::with_capacity(vertex_count * 3),
            normals: Vec::with_capacity(vertex_count * 3),
            indices: Vec::with_capacity(index_count),
        }
    }

    /// Check if mesh is empty
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Get triangle count
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Merge another mesh into this one
    pub fn merge(&mut self, other: &MeshData) {
        let vertex_offset = self.vertex_count() as u32;

        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.indices
            .extend(other.indices.iter().map(|i| i + vertex_offset));
    }
}

/// Model metadata extracted from IFC header
#[derive(Clone, Debug, Default)]
pub struct ModelMetadata {
    /// IFC schema version (e.g., "IFC2X3", "IFC4", "IFC4X3")
    pub schema_version: String,
    /// Originating system (CAD application)
    pub originating_system: Option<String>,
    /// Preprocessor version
    pub preprocessor_version: Option<String>,
    /// File name from header
    pub file_name: Option<String>,
    /// File description
    pub file_description: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Organization
    pub organization: Option<String>,
    /// Timestamp
    pub timestamp: Option<String>,
}
