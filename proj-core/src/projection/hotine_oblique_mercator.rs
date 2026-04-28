use crate::ellipsoid::Ellipsoid;
use crate::error::{Error, Result};
use crate::projection::{
    ensure_finite_lon_lat, ensure_finite_xy, normalize_longitude, validate_angle,
    validate_latitude_param, validate_lon_lat, validate_offset, validate_projected, validate_scale,
};

const POLE_EPSILON: f64 = 1e-12;
const ECCENTRICITY_EPSILON: f64 = 1e-15;

/// Hotine Oblique Mercator / Rectified Skew Orthomorphic projection.
///
/// Implements EPSG methods 9812 (variant A) and 9815 (variant B). Variant B
/// uses easting/northing at the projection centre and applies the centre-line
/// `u` offset; variant A uses false easting/northing at the natural origin.
pub(crate) struct HotineObliqueMercator {
    e: f64,
    a_const: f64,
    b: f64,
    h: f64,
    gamma0: f64,
    gamma_c: f64,
    lon0: f64,
    u_c: f64,
    false_easting: f64,
    false_northing: f64,
}

impl HotineObliqueMercator {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        ellipsoid: Ellipsoid,
        latc: f64,
        lonc: f64,
        azimuth: f64,
        rectified_grid_angle: f64,
        k0: f64,
        false_easting: f64,
        false_northing: f64,
        variant_b: bool,
    ) -> Result<Self> {
        validate_latitude_param("latitude of projection centre", latc)?;
        validate_angle("longitude of projection centre", lonc)?;
        validate_angle("azimuth of central line", azimuth)?;
        validate_angle("rectified grid angle", rectified_grid_angle)?;
        validate_scale("scale factor", k0)?;
        validate_offset("false easting", false_easting)?;
        validate_offset("false northing", false_northing)?;
        if (latc.abs() - std::f64::consts::FRAC_PI_2).abs() < POLE_EPSILON {
            return Err(Error::InvalidDefinition(
                "Hotine Oblique Mercator projection centre cannot be at a pole".into(),
            ));
        }

        let e2 = ellipsoid.e2();
        let e = ellipsoid.e();
        let sin_latc = latc.sin();
        let cos_latc = latc.cos();
        let b = (1.0 + e2 * cos_latc.powi(4) / (1.0 - e2)).sqrt();
        let a_const = ellipsoid.a * b * k0 * (1.0 - e2).sqrt() / (1.0 - e2 * sin_latc * sin_latc);
        let t0 = t_func(latc, e);
        let d = b * (1.0 - e2).sqrt() / (cos_latc * (1.0 - e2 * sin_latc * sin_latc).sqrt());
        let d_sq = (d * d).max(1.0);
        let f = d + (d_sq - 1.0).sqrt() * latc.signum();
        if !f.is_finite() || f <= 0.0 {
            return Err(Error::InvalidDefinition(
                "Hotine Oblique Mercator origin constants are invalid".into(),
            ));
        }
        let h = f * t0.powf(b);
        let g = (f - 1.0 / f) / 2.0;
        let gamma0 = (azimuth.sin() / d).clamp(-1.0, 1.0).asin();
        let lon0 = lonc - (g * gamma0.tan()).clamp(-1.0, 1.0).asin() / b;

        let u_c = if variant_b {
            (a_const / b) * (d_sq - 1.0).sqrt().atan2(azimuth.cos()) * latc.signum()
        } else {
            0.0
        };

        Ok(Self {
            e,
            a_const,
            b,
            h,
            gamma0,
            gamma_c: rectified_grid_angle,
            lon0,
            u_c,
            false_easting,
            false_northing,
        })
    }
}

fn t_func(lat: f64, e: f64) -> f64 {
    if e.abs() < ECCENTRICITY_EPSILON {
        return (std::f64::consts::FRAC_PI_4 - lat / 2.0).tan();
    }

    let sin_lat = lat.sin();
    let e_sin = e * sin_lat;
    (std::f64::consts::FRAC_PI_4 - lat / 2.0).tan() / ((1.0 - e_sin) / (1.0 + e_sin)).powf(e / 2.0)
}

fn lat_from_t(t: f64, e: f64) -> f64 {
    if e.abs() < ECCENTRICITY_EPSILON {
        return std::f64::consts::FRAC_PI_2 - 2.0 * t.atan();
    }

    let mut lat = std::f64::consts::FRAC_PI_2 - 2.0 * t.atan();
    for _ in 0..15 {
        let e_sin = e * lat.sin();
        let new_lat = std::f64::consts::FRAC_PI_2
            - 2.0 * (t * ((1.0 - e_sin) / (1.0 + e_sin)).powf(e / 2.0)).atan();
        if (new_lat - lat).abs() < 1e-14 {
            return new_lat;
        }
        lat = new_lat;
    }
    lat
}

impl super::ProjectionImpl for HotineObliqueMercator {
    fn forward(&self, lon: f64, lat: f64) -> Result<(f64, f64)> {
        validate_lon_lat(lon, lat)?;
        let d_lon = normalize_longitude(lon - self.lon0);
        let b_d_lon = self.b * d_lon;
        let t = t_func(lat, self.e);
        let q = self.h / t.powf(self.b);
        let s = (q - 1.0 / q) / 2.0;
        let t_h = (q + 1.0 / q) / 2.0;
        let v = b_d_lon.sin();
        let u_factor = (-v * self.gamma0.cos() + s * self.gamma0.sin()) / t_h;
        if u_factor.abs() >= 1.0 {
            return Err(Error::OutOfRange(
                "Hotine Oblique Mercator is undefined for this coordinate".into(),
            ));
        }

        let skew_v = self.a_const * ((1.0 - u_factor) / (1.0 + u_factor)).ln() / (2.0 * self.b);
        let skew_u = self.a_const
            * (s * self.gamma0.cos() + v * self.gamma0.sin()).atan2(b_d_lon.cos())
            / self.b
            - self.u_c;

        let x = self.false_easting + skew_v * self.gamma_c.cos() + skew_u * self.gamma_c.sin();
        let y = self.false_northing + skew_u * self.gamma_c.cos() - skew_v * self.gamma_c.sin();

        ensure_finite_xy("Hotine Oblique Mercator", x, y)
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        validate_projected(x, y)?;
        let dx = x - self.false_easting;
        let dy = y - self.false_northing;
        let skew_v = dx * self.gamma_c.cos() - dy * self.gamma_c.sin();
        let skew_u = dy * self.gamma_c.cos() + dx * self.gamma_c.sin() + self.u_c;

        let q = (-self.b * skew_v / self.a_const).exp();
        let s = (q - 1.0 / q) / 2.0;
        let t_h = (q + 1.0 / q) / 2.0;
        let v = (self.b * skew_u / self.a_const).sin();
        let u = (v * self.gamma0.cos() + s * self.gamma0.sin()) / t_h;
        if u.abs() >= 1.0 {
            return Err(Error::OutOfRange(
                "Hotine Oblique Mercator inverse is undefined for this coordinate".into(),
            ));
        }

        let t = (self.h / ((1.0 + u) / (1.0 - u)).sqrt()).powf(1.0 / self.b);
        let lat = lat_from_t(t, self.e);
        let lon = self.lon0
            - (s * self.gamma0.cos() - v * self.gamma0.sin())
                .atan2((self.b * skew_u / self.a_const).cos())
                / self.b;

        ensure_finite_lon_lat("Hotine Oblique Mercator", lon, lat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ellipsoid::Ellipsoid;
    use crate::projection::ProjectionImpl;

    fn dms(deg: f64, min: f64, sec: f64) -> f64 {
        deg + min / 60.0 + sec / 3600.0
    }

    fn everest_1830_1967() -> Ellipsoid {
        Ellipsoid::from_a_rf(6_377_298.556, 300.8017)
    }

    #[test]
    fn epsg_variant_a_example() {
        let proj = HotineObliqueMercator::new(
            everest_1830_1967(),
            dms(4.0, 0.0, 0.0).to_radians(),
            dms(115.0, 0.0, 0.0).to_radians(),
            dms(53.0, 18.0, 56.9537).to_radians(),
            dms(53.0, 7.0, 48.3685).to_radians(),
            0.99984,
            0.0,
            0.0,
            false,
        )
        .unwrap();

        let lon = dms(115.0, 48.0, 19.8196).to_radians();
        let lat = dms(5.0, 23.0, 14.1129).to_radians();
        let (x, y) = proj.forward(lon, lat).unwrap();

        assert!((x - 679_245.73).abs() < 0.02, "x = {x}");
        assert!((y - 596_562.78).abs() < 0.02, "y = {y}");

        let (lon2, lat2) = proj.inverse(x, y).unwrap();
        assert!((lon2 - lon).abs() < 1e-8);
        assert!((lat2 - lat).abs() < 1e-8);
    }

    #[test]
    fn epsg_variant_b_example() {
        let proj = HotineObliqueMercator::new(
            everest_1830_1967(),
            dms(4.0, 0.0, 0.0).to_radians(),
            dms(115.0, 0.0, 0.0).to_radians(),
            dms(53.0, 18.0, 56.9537).to_radians(),
            dms(53.0, 7.0, 48.3685).to_radians(),
            0.99984,
            590_476.87,
            442_857.65,
            true,
        )
        .unwrap();

        let lon = dms(115.0, 48.0, 19.8196).to_radians();
        let lat = dms(5.0, 23.0, 14.1129).to_radians();
        let (x, y) = proj.forward(lon, lat).unwrap();

        assert!((x - 679_245.73).abs() < 0.02, "x = {x}");
        assert!((y - 596_562.78).abs() < 0.02, "y = {y}");

        let (lon2, lat2) = proj.inverse(x, y).unwrap();
        assert!((lon2 - lon).abs() < 1e-8);
        assert!((lat2 - lat).abs() < 1e-8);
    }
}
