use crate::coord::{Bounds, Coord};
use crate::crs::{LinearUnit, ProjectionMethod};
use crate::datum::{DatumToWgs84, HelmertParams};
use smallvec::SmallVec;
use std::collections::HashSet;
use std::sync::Arc;

/// Stable identifier for a registry-backed coordinate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoordinateOperationId(pub u32);

/// Stable identifier for a grid resource referenced by an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridId(pub u32);

/// Ranked area-of-use metadata for an operation or grid.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaOfUse {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
    pub name: String,
}

impl AreaOfUse {
    pub fn contains_point(&self, point: Coord) -> bool {
        point.x >= self.west
            && point.x <= self.east
            && point.y >= self.south
            && point.y <= self.north
    }

    pub fn contains_bounds(&self, bounds: Bounds) -> bool {
        bounds.min_x >= self.west
            && bounds.max_x <= self.east
            && bounds.min_y >= self.south
            && bounds.max_y <= self.north
    }
}

/// Nominal operation accuracy in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OperationAccuracy {
    pub meters: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStepDirection {
    Forward,
    Reverse,
}

impl OperationStepDirection {
    pub fn inverse(self) -> Self {
        match self {
            Self::Forward => Self::Reverse,
            Self::Reverse => Self::Forward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaOfInterestCrs {
    GeographicDegrees,
    SourceCrs,
    TargetCrs,
}

impl AreaOfInterestCrs {
    pub fn inverse(self) -> Self {
        match self {
            Self::GeographicDegrees => Self::GeographicDegrees,
            Self::SourceCrs => Self::TargetCrs,
            Self::TargetCrs => Self::SourceCrs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaOfInterest {
    pub crs: AreaOfInterestCrs,
    pub point: Option<Coord>,
    pub bounds: Option<Bounds>,
}

impl AreaOfInterest {
    pub fn geographic_point(point: Coord) -> Self {
        Self {
            crs: AreaOfInterestCrs::GeographicDegrees,
            point: Some(point),
            bounds: None,
        }
    }

    pub fn geographic_bounds(bounds: Bounds) -> Self {
        Self {
            crs: AreaOfInterestCrs::GeographicDegrees,
            point: None,
            bounds: Some(bounds),
        }
    }

    pub fn source_crs_point(point: Coord) -> Self {
        Self {
            crs: AreaOfInterestCrs::SourceCrs,
            point: Some(point),
            bounds: None,
        }
    }

    pub fn source_crs_bounds(bounds: Bounds) -> Self {
        Self {
            crs: AreaOfInterestCrs::SourceCrs,
            point: None,
            bounds: Some(bounds),
        }
    }

    pub fn target_crs_point(point: Coord) -> Self {
        Self {
            crs: AreaOfInterestCrs::TargetCrs,
            point: Some(point),
            bounds: None,
        }
    }

    pub fn target_crs_bounds(bounds: Bounds) -> Self {
        Self {
            crs: AreaOfInterestCrs::TargetCrs,
            point: None,
            bounds: Some(bounds),
        }
    }

    pub fn inverse(self) -> Self {
        Self {
            crs: self.crs.inverse(),
            point: self.point,
            bounds: self.bounds,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridInterpolation {
    Bilinear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridShiftDirection {
    Forward,
    Reverse,
}

impl GridShiftDirection {
    pub fn inverse(self) -> Self {
        match self {
            Self::Forward => Self::Reverse,
            Self::Reverse => Self::Forward,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperationStep {
    pub operation_id: CoordinateOperationId,
    pub direction: OperationStepDirection,
}

/// Enum-backed operation method model used by selection and compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum OperationMethod {
    Identity,
    Helmert {
        params: HelmertParams,
    },
    GridShift {
        grid_id: GridId,
        interpolation: GridInterpolation,
        direction: GridShiftDirection,
    },
    DatumShift {
        source_to_wgs84: DatumToWgs84,
        target_to_wgs84: DatumToWgs84,
    },
    Projection {
        forward: bool,
        method: ProjectionMethod,
        linear_unit: LinearUnit,
    },
    AxisUnitNormalize,
    Concatenated {
        steps: SmallVec<[OperationStep; 4]>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMatchKind {
    ExactSourceTarget,
    DerivedGeographic,
    DatumCompatible,
    ApproximateFallback,
    Explicit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateOperation {
    pub id: Option<CoordinateOperationId>,
    pub name: String,
    pub source_crs_epsg: Option<u32>,
    pub target_crs_epsg: Option<u32>,
    pub source_datum_epsg: Option<u32>,
    pub target_datum_epsg: Option<u32>,
    pub accuracy: Option<OperationAccuracy>,
    pub areas_of_use: SmallVec<[AreaOfUse; 1]>,
    pub deprecated: bool,
    pub preferred: bool,
    pub approximate: bool,
    pub method: OperationMethod,
}

impl CoordinateOperation {
    pub fn metadata(&self) -> CoordinateOperationMetadata {
        CoordinateOperationMetadata {
            id: self.id,
            name: self.name.clone(),
            direction: OperationStepDirection::Forward,
            source_crs_epsg: self.source_crs_epsg,
            target_crs_epsg: self.target_crs_epsg,
            source_datum_epsg: self.source_datum_epsg,
            target_datum_epsg: self.target_datum_epsg,
            accuracy: self.accuracy,
            area_of_use: self.areas_of_use.first().cloned(),
            deprecated: self.deprecated,
            preferred: self.preferred,
            approximate: self.approximate,
            uses_grids: self.uses_grids(),
        }
    }

    pub fn metadata_for_direction(
        &self,
        direction: OperationStepDirection,
    ) -> CoordinateOperationMetadata {
        let mut metadata = self.metadata();
        metadata.direction = direction;
        if matches!(direction, OperationStepDirection::Reverse) {
            std::mem::swap(&mut metadata.source_crs_epsg, &mut metadata.target_crs_epsg);
            std::mem::swap(
                &mut metadata.source_datum_epsg,
                &mut metadata.target_datum_epsg,
            );
        }
        metadata
    }

    pub fn uses_grids(&self) -> bool {
        let mut visited = HashSet::new();
        self.uses_grids_with_visited(&mut visited)
    }

    fn uses_grids_with_visited(&self, visited: &mut HashSet<CoordinateOperationId>) -> bool {
        match &self.method {
            OperationMethod::GridShift { .. } => true,
            OperationMethod::DatumShift {
                source_to_wgs84,
                target_to_wgs84,
            } => source_to_wgs84.uses_grid_shift() || target_to_wgs84.uses_grid_shift(),
            OperationMethod::Concatenated { steps } => steps.iter().any(|step| {
                if !visited.insert(step.operation_id) {
                    return false;
                }
                let uses_grids = crate::registry::lookup_operation(step.operation_id)
                    .map(|operation| operation.uses_grids_with_visited(visited))
                    .unwrap_or(false);
                visited.remove(&step.operation_id);
                uses_grids
            }),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateOperationMetadata {
    pub id: Option<CoordinateOperationId>,
    pub name: String,
    pub direction: OperationStepDirection,
    pub source_crs_epsg: Option<u32>,
    pub target_crs_epsg: Option<u32>,
    pub source_datum_epsg: Option<u32>,
    pub target_datum_epsg: Option<u32>,
    pub accuracy: Option<OperationAccuracy>,
    pub area_of_use: Option<AreaOfUse>,
    pub deprecated: bool,
    pub preferred: bool,
    pub approximate: bool,
    pub uses_grids: bool,
}

#[derive(Debug, Clone)]
pub enum SelectionPolicy {
    BestAvailable,
    RequireGrids,
    RequireExactAreaMatch,
    AllowApproximateHelmertFallback,
    Operation(CoordinateOperationId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalGridOffsetConvention {
    /// Grid values are geoid heights in meters (`N`), applied as
    /// gravity height `H = h - N` and ellipsoidal height `h = H + N`.
    GeoidHeightMeters,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalGridOperation {
    /// Human-readable operation name used in diagnostics.
    pub name: String,
    /// Grid resource definition resolved through the configured grid provider.
    pub grid: crate::grid::GridDefinition,
    /// Horizontal CRS EPSG code in which the grid is sampled, when known.
    pub grid_horizontal_crs_epsg: Option<u32>,
    /// Optional source vertical CRS EPSG filter.
    pub source_vertical_crs_epsg: Option<u32>,
    /// Optional target vertical CRS EPSG filter.
    pub target_vertical_crs_epsg: Option<u32>,
    /// Optional source gravity-related vertical datum EPSG filter.
    pub source_vertical_datum_epsg: Option<u32>,
    /// Optional target gravity-related vertical datum EPSG filter.
    pub target_vertical_datum_epsg: Option<u32>,
    /// Expected operation accuracy in meters, when known.
    pub accuracy: Option<OperationAccuracy>,
    /// Operation area of use, when distinct from the grid's area.
    pub area_of_use: Option<AreaOfUse>,
    pub offset_convention: VerticalGridOffsetConvention,
}

impl VerticalGridOperation {
    pub fn inverse(&self) -> Self {
        let mut inverse = self.clone();
        std::mem::swap(
            &mut inverse.source_vertical_crs_epsg,
            &mut inverse.target_vertical_crs_epsg,
        );
        std::mem::swap(
            &mut inverse.source_vertical_datum_epsg,
            &mut inverse.target_vertical_datum_epsg,
        );
        inverse
    }
}

#[derive(Clone)]
pub struct SelectionOptions {
    pub area_of_interest: Option<AreaOfInterest>,
    pub policy: SelectionPolicy,
    pub grid_provider: Option<Arc<dyn crate::grid::GridProvider>>,
    pub vertical_grid_operations: Vec<VerticalGridOperation>,
}

impl Default for SelectionOptions {
    fn default() -> Self {
        Self {
            area_of_interest: None,
            policy: SelectionPolicy::BestAvailable,
            grid_provider: None,
            vertical_grid_operations: Vec::new(),
        }
    }
}

impl SelectionOptions {
    pub fn inverse(&self) -> Self {
        Self {
            area_of_interest: self.area_of_interest.map(AreaOfInterest::inverse),
            policy: self.policy.clone(),
            grid_provider: self.grid_provider.clone(),
            vertical_grid_operations: self
                .vertical_grid_operations
                .iter()
                .map(VerticalGridOperation::inverse)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionReason {
    ExplicitOperation,
    ExactSourceTarget,
    AreaOfUseMatch,
    AccuracyPreferred,
    NonDeprecated,
    PreferredOperation,
    ApproximateFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkippedOperationReason {
    AreaOfUseMismatch,
    MissingGrid,
    UnsupportedGridFormat,
    PolicyFiltered,
    LessPreferred,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkippedOperation {
    pub metadata: CoordinateOperationMetadata,
    pub reason: SkippedOperationReason,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalTransformAction {
    /// No explicit vertical CRS participates in the transform.
    None,
    /// `z` is preserved because the vertical CRS semantics and units match.
    Preserved,
    /// `z` is converted between units of the same vertical reference frame.
    UnitConverted,
    /// `z` is transformed by an explicit vertical operation.
    Transformed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalGridProvenance {
    pub name: String,
    pub checksum: Option<String>,
    pub accuracy: Option<OperationAccuracy>,
    pub area_of_use: Option<AreaOfUse>,
    pub area_of_use_match: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalTransformDiagnostics {
    pub action: VerticalTransformAction,
    pub operation_name: Option<String>,
    pub source_vertical_crs_epsg: Option<u32>,
    pub target_vertical_crs_epsg: Option<u32>,
    pub source_vertical_datum_epsg: Option<u32>,
    pub target_vertical_datum_epsg: Option<u32>,
    pub source_unit_to_meter: Option<f64>,
    pub target_unit_to_meter: Option<f64>,
    pub accuracy: Option<OperationAccuracy>,
    pub area_of_use: Option<AreaOfUse>,
    pub area_of_use_match: Option<bool>,
    pub grids: Vec<VerticalGridProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperationSelectionDiagnostics {
    pub selected_operation: CoordinateOperationMetadata,
    pub selected_match_kind: OperationMatchKind,
    pub selected_reasons: SmallVec<[SelectionReason; 4]>,
    pub fallback_operations: Vec<CoordinateOperationMetadata>,
    pub skipped_operations: Vec<SkippedOperation>,
    pub approximate: bool,
    pub missing_required_grid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridCoverageMiss {
    pub operation: CoordinateOperationMetadata,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformOutcome<T> {
    pub coord: T,
    pub operation: CoordinateOperationMetadata,
    pub vertical: VerticalTransformDiagnostics,
    pub grid_coverage_misses: Vec<GridCoverageMiss>,
}
