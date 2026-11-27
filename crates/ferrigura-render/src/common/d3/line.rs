//! `line()` of `d3-shape` 3.2.0 with its linear curve (`curve/linear.js`) and the three-digit path
//! writer of `d3-path` (`path.js`).

use crate::common::d3::d3_number;

/// The path of `line()(points)`: `M` to the first point, `L` to each other, with each coordinate
/// rounded to three digits (`Math.round(v * 1000) / 1000`). A single point is closed with `Z`, as
/// the linear curve does. No points give no path, which the reference returns as `null`.
///
/// A coordinate that is NaN is written as `NaN`, as `+x(d)` gives NaN for an undefined value.
#[must_use]
pub fn linear_path(points: &[(f64, f64)]) -> Option<String> {
    if points.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (index, (x, y)) in points.iter().enumerate() {
        out.push(if index == 0 { 'M' } else { 'L' });
        out.push_str(&d3_number(*x));
        out.push(',');
        out.push_str(&d3_number(*y));
    }
    if points.len() == 1 {
        out.push('Z');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_points_give_no_path() {
        assert_eq!(linear_path(&[]), None);
    }

    #[test]
    fn a_single_point_is_closed() {
        assert_eq!(
            linear_path(&[(361.32, 255.5)]).as_deref(),
            Some("M361.32,255.5Z")
        );
    }

    #[test]
    fn coordinates_are_rounded_to_three_digits() {
        assert_eq!(
            linear_path(&[(43.6949, 457.5), (367.0551, 8.5), (690.41449, 233.0)]).as_deref(),
            Some("M43.695,457.5L367.055,8.5L690.414,233")
        );
    }

    #[test]
    fn a_large_or_small_coordinate_is_written_as_js_writes_it() {
        assert_eq!(
            linear_path(&[(1e21, 1.5e-7), (-2.5, 1.0 / 3.0)]).as_deref(),
            Some("M1e+21,0L-2.5,0.333")
        );
    }

    #[test]
    fn a_missing_coordinate_is_written_as_nan() {
        assert_eq!(
            linear_path(&[(40.141, f64::NAN), (40.141, 233.0)]).as_deref(),
            Some("M40.141,NaNL40.141,233")
        );
    }
}
