use std::f64::consts::{PI, TAU};

type LatitudeRad = f64;
type LongitudeRad = f64;
type LatitudeDeg = f64;
type LongitudeDeg = f64;
type AngleRad = f64;
pub type JulianDate = f64;

#[derive(Default)]
pub struct AngleHour {
    pub hour: i64,
    pub minute: i64,
    pub second: f64,
}

impl From<AngleRad> for AngleHour {
    fn from(angle: AngleRad) -> Self {
        let angle = (angle * 12.0 / PI) % 24.0;
        let hour = angle.floor() as i64;
        let minute = ((angle - hour as f64) * 60.0).floor() as i64;
        let second = (angle - hour as f64 - minute as f64 / 60.0) * 3600.0;
        AngleHour {
            hour,
            minute,
            second,
        }
    }
}

impl From<AngleHour> for AngleRad {
    fn from(angle: AngleHour) -> Self {
        let angle = angle.hour as f64 + angle.minute as f64 / 60.0 + angle.second / 3600.0;
        (angle * PI / 12.0) % TAU
    }
}

#[derive(Default)]
pub struct SunPosition {
    /// Right ascension in radians
    pub ra: AngleRad,
    /// Declination in radians
    pub dec: AngleRad,
    /// Distance from Earth
    pub r: f64,
}

#[derive(Clone, Copy, Default)]
pub struct EarthCoordsRad {
    /// Latitude in radians
    pub lat: LatitudeRad,
    /// Longitude in radians
    pub lon: LongitudeRad,
}

#[derive(Clone, Copy, Default)]
pub struct EarthCoordsDeg {
    /// Latitude in degrees
    pub lat: LatitudeDeg,
    /// Longitude in degrees
    pub lon: LongitudeDeg,
}

impl From<EarthCoordsDeg> for EarthCoordsRad {
    fn from(coords: EarthCoordsDeg) -> Self {
        EarthCoordsRad {
            lat: coords.lat.to_radians(),
            lon: coords.lon.to_radians(),
        }
    }
}

impl From<EarthCoordsRad> for EarthCoordsDeg {
    fn from(coords: EarthCoordsRad) -> Self {
        EarthCoordsDeg {
            lat: coords.lat.to_degrees(),
            lon: coords.lon.to_degrees(),
        }
    }
}

pub fn get_sun_position(julian_date: JulianDate) -> SunPosition {
    let t: f64 = (julian_date - 2451545.0) / 365250.0;
    let mut y: f64 = 0.00010466965 * (0.09641690558 + 18849.2275499742 * t).cos();
    y += 0.00835292314 * (0.13952878991 + 12566.1516999828 * t).cos() - 0.02442699036;
    y += 0.99989211030 * (0.18265890456 + 6283.07584999140 * t).cos();
    y += (0.00093046324 + 0.00051506609 * (4.43180499286 + 12566.1516999828 * t).cos()) * t;

    let mut x: f64 = 0.00561144206 + 0.00010466628 * (1.66722645223 + 18849.2275499742 * t).cos();
    x += 0.00835257300 * (1.71034539450 + 12566.1516999828 * t).cos();
    x += 0.99982928844 * (1.75348568475 + 6283.07584999140 * t).cos();
    x += (0.00123403056 + 0.00051500156 * (6.00266267204 + 12566.1516999828 * t).cos()) * t;

    let z: f64 = 0.00227822442 * (3.41372504278 + 6283.07584999140 * t).cos() * t;

    // Rotate from VSOP to J2000
    let tx: f64 = -(x + y * 0.000000440360 + z * -0.000000190919);
    let ty: f64 = -(x * -0.000000479966 + y * 0.917482137087 + z * -0.397776982902);
    let tz: f64 = -(y * 0.397776982902 + z * 0.917482137087);

    // Convert from Cartesian to polar coordinates
    let r: f64 = (tx * tx + ty * ty + tz * tz).sqrt();
    let mut ra: AngleRad = ty.atan2(tx);
    if ra < 0.0 {
        ra += TAU;
    }
    let mut dec: AngleRad = (tz / r).acos();
    dec = PI / 2.0 - dec;

    SunPosition { ra, dec, r }
}

/// Get Greenwich Mean Sidereal Time (GMST) in radians
pub fn get_gmst(jd_ut1: JulianDate) -> AngleRad {
    let t = (jd_ut1 - 2451545.0) / 36525.0;
    let era = get_earth_rotation_angle(jd_ut1);
    let gmst = (era
        + (0.014506
            + 4612.15739966 * t
            + 1.39667721 * t.powi(2)
            + -0.00009344 * t.powi(3)
            + 0.00001882 * t.powi(4))
            * PI
            / 648000.0)
        % TAU;
    if gmst < 0.0 {
        gmst + TAU
    } else {
        gmst
    }
}

impl EarthCoordsRad {
    pub fn from_ra_dec(ra: AngleRad, dec: AngleRad, jd_ut1: JulianDate) -> Self {
        let lon = ra - get_gmst(jd_ut1);
        let lat = dec;
        // clamp lon to [-PI, PI]
        let lon = if lon < -PI {
            lon + TAU
        } else if lon > PI {
            lon - TAU
        } else {
            lon
        };
        EarthCoordsRad { lat, lon }
    }
}

/// Get Earth rotation angle (ERA) in radians
fn get_earth_rotation_angle(jd_ut1: JulianDate) -> AngleRad {
    let t = jd_ut1 - 2451545.0;
    let frac = t % 1.0;
    let era: f64 = (TAU * (0.7790572732640 + 0.00273781191135448 * t + frac)) % TAU;
    if era < 0.0 {
        era + TAU
    } else {
        era
    }
}

pub fn get_terminator_point(lon: LongitudeRad, gp: &EarthCoordsRad) -> LatitudeRad {
    let ha = lon - gp.lon;
    (ha.cos() / gp.lat.tan()).atan()
}

pub fn get_terminator_outline(gp: &EarthCoordsRad) -> Vec<LatitudeRad> {
    let mut outline = Vec::new();
    for lon in -720..=720 {
        let lon = (lon as f64 / 4.0).to_radians();
        outline.push(get_terminator_point(lon, gp));
    }
    outline
}

/// Get the Julian Date from a Unix timestamp (in miliseconds)
pub fn julian_date_from_unix_timestamp(unix_timestamp: i64) -> JulianDate {
    (unix_timestamp as f64) / 86400000.0 + 2440587.5
}
