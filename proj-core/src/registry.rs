use crate::crs::CrsDef;
use crate::datum::Datum;
use crate::epsg_db;
use crate::error::{Error, Result};
use crate::grid::GridDefinition;
use crate::operation::{CoordinateOperation, CoordinateOperationId};

/// Look up a CRS definition by EPSG code.
///
/// Searches the embedded EPSG database (~5,600 CRS definitions) covering all
/// geographic 2D CRS and all projected CRS that use supported projection methods.
///
/// Returns `None` if the EPSG code is not in the database.
pub fn lookup_epsg(code: u32) -> Option<CrsDef> {
    epsg_db::lookup(code)
}

/// Look up a datum definition by EPSG code.
pub fn lookup_datum_epsg(code: u32) -> Option<Datum> {
    epsg_db::lookup_datum(code)
}

/// Look up a coordinate operation by its identifier.
pub fn lookup_operation(id: CoordinateOperationId) -> Option<CoordinateOperation> {
    epsg_db::lookup_operation(id.0)
}

/// Look up a grid definition by its identifier.
pub(crate) fn lookup_grid_definition(id: u32) -> Option<GridDefinition> {
    epsg_db::lookup_grid(id)
}

pub(crate) fn all_operations() -> Vec<CoordinateOperation> {
    epsg_db::operations()
}

/// Return all registry operations compatible with the source and target CRS.
pub fn operations_between(source: &CrsDef, target: &CrsDef) -> Vec<CoordinateOperation> {
    let source_geo = source.base_geographic_crs_epsg();
    let target_geo = target.base_geographic_crs_epsg();
    let source_datum = source_geo.and_then(epsg_db::lookup_datum_code_for_crs);
    let target_datum = target_geo.and_then(epsg_db::lookup_datum_code_for_crs);
    epsg_db::operations()
        .into_iter()
        .filter(|operation| match (source_geo, target_geo) {
            (Some(source_code), Some(target_code)) => {
                (operation.source_crs_epsg == Some(source_code)
                    && operation.target_crs_epsg == Some(target_code))
                    || (source_datum.is_some()
                        && target_datum.is_some()
                        && operation.source_datum_epsg == source_datum
                        && operation.target_datum_epsg == target_datum)
            }
            _ => false,
        })
        .collect()
}

/// Parse an authority:code string (e.g., "EPSG:4326") and look up the CRS definition.
///
/// Currently only supports the "EPSG" authority.
pub fn lookup_authority_code(code: &str) -> Result<CrsDef> {
    let Some((authority, code_str)) = code.split_once(':') else {
        return Err(Error::UnknownCrs(format!(
            "invalid authority:code format: {code}"
        )));
    };

    let authority = authority.trim();
    let code_str = code_str.trim();

    if !authority.eq_ignore_ascii_case("EPSG") {
        return Err(Error::UnknownCrs(format!(
            "unsupported authority: {authority} (only EPSG is supported)"
        )));
    }

    let epsg: u32 = code_str
        .parse()
        .map_err(|_| Error::UnknownCrs(format!("invalid EPSG code: {code_str}")))?;

    lookup_epsg(epsg).ok_or_else(|| Error::UnknownCrs(format!("unknown EPSG code: {epsg}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_wgs84() {
        let crs = lookup_epsg(4326).expect("should find 4326");
        assert!(crs.is_geographic());
        assert_eq!(crs.epsg(), 4326);
    }

    #[test]
    fn lookup_web_mercator() {
        let crs = lookup_epsg(3857).expect("should find 3857");
        assert!(crs.is_projected());
        assert_eq!(crs.epsg(), 3857);
    }

    #[test]
    fn lookup_polar_stereo_north() {
        let crs = lookup_epsg(3413).expect("should find 3413");
        assert!(crs.is_projected());
    }

    #[test]
    fn lookup_utm_zone_18n() {
        let crs = lookup_epsg(32618).expect("should find UTM 18N");
        assert!(crs.is_projected());
        assert_eq!(crs.epsg(), 32618);
    }

    #[test]
    fn lookup_utm_zone_1s() {
        let crs = lookup_epsg(32701).expect("should find UTM 1S");
        assert!(crs.is_projected());
        assert_eq!(crs.epsg(), 32701);
    }

    #[test]
    fn lookup_utm_zone_60n() {
        let crs = lookup_epsg(32660).expect("should find UTM 60N");
        assert!(crs.is_projected());
    }

    #[test]
    fn lookup_unknown_epsg() {
        assert!(lookup_epsg(99999).is_none());
    }

    #[test]
    fn authority_code_parse() {
        let crs = lookup_authority_code("EPSG:4326").expect("should parse");
        assert_eq!(crs.epsg(), 4326);
    }

    #[test]
    fn authority_code_case_insensitive() {
        let crs = lookup_authority_code("epsg:3857").expect("should parse");
        assert_eq!(crs.epsg(), 3857);
    }

    #[test]
    fn authority_code_invalid_format() {
        assert!(lookup_authority_code("NONSENSE").is_err());
    }

    #[test]
    fn authority_code_unknown() {
        assert!(lookup_authority_code("EPSG:99999").is_err());
    }

    #[test]
    fn authority_code_non_epsg() {
        assert!(lookup_authority_code("OGC:CRS84").is_err());
    }

    #[test]
    fn nad27_lookup() {
        let crs = lookup_epsg(4267).expect("should find NAD27");
        assert!(crs.is_geographic());
    }

    #[test]
    fn new_zealand_tm() {
        let crs = lookup_epsg(2193).expect("should find NZTM 2000");
        assert!(crs.is_projected());
    }

    #[test]
    fn nc_state_plane() {
        let crs = lookup_epsg(32119).expect("should find NC State Plane");
        assert!(crs.is_projected());
    }
}
