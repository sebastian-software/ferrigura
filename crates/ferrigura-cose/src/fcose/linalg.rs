//! Dense linear algebra of layout-base 2.0.1: `util/Matrix.js` and `util/SVD.js`, used by the
//! spectral start (`spectral.rs`) and by the orthogonal Procrustes fit of cose-base 2.2.0.
//!
//! Matrices are `Vec<Vec<f64>>` rows, as in the source (`m[i][j]`). Every sum starts from `0`
//! and adds the terms in the source's loop order, because floating-point addition is not
//! associative. Each expression keeps the source's operand order.

// The index loops follow the source's loops over rows and columns (Matrix.js, SVD.js), so
// the iterator forms would hide the correspondence.
#![allow(clippy::needless_range_loop)]

use crate::fcose::Error;
use crate::geometry::js_max;

/// `Matrix.multMat` (Matrix.js 9-26): `array1 * array2`. The inner sum runs over `k` in order.
/// Both matrices are non-empty; the callers guarantee it (the source reads `array[0].length`).
pub(crate) fn mult_mat(array1: &[Vec<f64>], array2: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let cols = array2[0].len();
    let inner = array1[0].len();
    let mut result = Vec::with_capacity(array1.len());
    for i in 0..array1.len() {
        let mut row = Vec::with_capacity(cols);
        for j in 0..cols {
            let mut sum = 0.0;
            for k in 0..inner {
                sum += array1[i][k] * array2[k][j];
            }
            row.push(sum);
        }
        result.push(row);
    }
    result
}

/// `Matrix.transpose` (Matrix.js 28-43).
pub(crate) fn transpose(array: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let cols = array[0].len();
    let mut result = Vec::with_capacity(cols);
    for i in 0..cols {
        result.push((0..array.len()).map(|j| array[j][i]).collect());
    }
    result
}

/// `Matrix.multCons` (Matrix.js 45-57): every element times the constant.
pub(crate) fn mult_cons(array: &[f64], constant: f64) -> Vec<f64> {
    array.iter().map(|&x| x * constant).collect()
}

/// `Matrix.minusOp` (Matrix.js 59-71): element-wise difference.
pub(crate) fn minus_op(array1: &[f64], array2: &[f64]) -> Vec<f64> {
    array1.iter().zip(array2).map(|(&a, &b)| a - b).collect()
}

/// `Matrix.dotProduct` (Matrix.js 73-85). Runs over the length of `array1`, as the source does.
pub(crate) fn dot_product(array1: &[f64], array2: &[f64]) -> f64 {
    let mut product = 0.0;
    for i in 0..array1.len() {
        product += array1[i] * array2[i];
    }
    product
}

/// `Matrix.mag` (Matrix.js 87-89).
fn mag(array: &[f64]) -> f64 {
    dot_product(array, array).sqrt()
}

/// `Matrix.normalize` (Matrix.js 95-108): each element divided by the magnitude.
pub(crate) fn normalize(array: &[f64]) -> Vec<f64> {
    let magnitude = mag(array);
    array.iter().map(|&x| x / magnitude).collect()
}

/// `Matrix.multGamma` (Matrix.js 110-129): centres a vector. The mean is taken as
/// `sum * ((-1) / length)`, not as `-sum / length`, which rounds differently.
pub(crate) fn mult_gamma(array: &[f64]) -> Vec<f64> {
    let mut sum = 0.0;
    for &x in array {
        sum += x;
    }
    sum *= -1.0 / array.len() as f64;
    array.iter().map(|&x| sum + x).collect()
}

/// `Matrix.multL` (Matrix.js 131-164): `C * INV * C'` applied to a vector, in three passes. The
/// first pass takes its dimensions from `C[0]`, the second from `INV`, the third from `C`.
pub(crate) fn mult_l(array: &[f64], c: &[Vec<f64>], inv: &[Vec<f64>]) -> Vec<f64> {
    let cols = c[0].len();
    let mut temp1 = Vec::with_capacity(cols);
    for i in 0..cols {
        let mut sum = 0.0;
        for j in 0..c.len() {
            sum += -0.5 * c[j][i] * array[j];
        }
        temp1.push(sum);
    }
    let mut temp2 = Vec::with_capacity(inv.len());
    for i in 0..inv.len() {
        let mut sum = 0.0;
        for j in 0..inv.len() {
            sum += inv[i][j] * temp1[j];
        }
        temp2.push(sum);
    }
    let mut result = Vec::with_capacity(c.len());
    for i in 0..c.len() {
        let mut sum = 0.0;
        for j in 0..cols {
            sum += c[i][j] * temp2[j];
        }
        result.push(sum);
    }
    result
}

/// The result of `SVD.svd`: `A = U S V'`. `s` holds the singular values in decreasing order.
pub(crate) struct Svd {
    pub(crate) u: Vec<Vec<f64>>,
    pub(crate) v: Vec<Vec<f64>>,
    pub(crate) s: Vec<f64>,
}

/// `SVD.hypot` (SVD.js 624-637). It is not `Math.hypot`: it scales by the larger operand and
/// returns 0 for two zeros.
fn svd_hypot(a: f64, b: f64) -> f64 {
    if a.abs() > b.abs() {
        let r = b / a;
        a.abs() * (1.0 + r * r).sqrt()
    } else if b != 0.0 {
        let r = a / b;
        b.abs() * (1.0 + r * r).sqrt()
    } else {
        0.0
    }
}

/// `SVD.svd` (SVD.js 216-622): the Householder bidiagonalisation and the implicit-shift QR
/// iteration of the Jama SVD, for an `m x n` matrix with `m` and `n` at least 1. The input is
/// consumed because the source reduces it in place. Both `wantu` and `wantv` are true in the
/// source (SVD.js 271-272), so their branches always run and are not written here.
///
/// Where the source takes `Math.min` or `Math.max` of counts, the code uses `min`, `max`, and
/// `saturating_sub` for the `-1` and `-2` of the source, which give the same bounds. The loop
/// `for (let k = nct - 1; k >= 0; k--)` runs as `(0..nct).rev()`. The source's `iter` counter
/// is never read, so it is not kept.
pub(crate) fn svd(mut a: Vec<Vec<f64>>) -> Svd {
    let m = a.len();
    let n = a[0].len();
    let nu = m.min(n);
    // SVD.js 225-228: `Math.min(this.m + 1, this.n)` zeros.
    let mut s = vec![0.0; (m + 1).min(n)];
    let mut u = vec![vec![0.0; nu]; m];
    let mut v = vec![vec![0.0; n]; n];
    let mut e = vec![0.0; n];
    let mut work = vec![0.0; m];
    // SVD.js 273-274.
    let nct = m.saturating_sub(1).min(n);
    let nrt = n.saturating_sub(2).min(m);

    // SVD.js 275-363: the reduction to bidiagonal form.
    for k in 0..nct.max(nrt) {
        if k < nct {
            s[k] = 0.0;
            for i in k..m {
                s[k] = svd_hypot(s[k], a[i][k]);
            }
            if s[k] != 0.0 {
                if a[k][k] < 0.0 {
                    s[k] = -s[k];
                }
                for i in k..m {
                    a[i][k] /= s[k];
                }
                a[k][k] += 1.0;
            }
            s[k] = -s[k];
        }
        for j in k + 1..n {
            if k < nct && s[k] != 0.0 {
                let mut t = 0.0;
                for i in k..m {
                    t += a[i][k] * a[i][j];
                }
                t = -t / a[k][k];
                for i in k..m {
                    a[i][j] += t * a[i][k];
                }
            }
            e[j] = a[k][j];
        }
        if k < nct {
            for i in k..m {
                u[i][k] = a[i][k];
            }
        }
        if k < nrt {
            e[k] = 0.0;
            for i in k + 1..n {
                e[k] = svd_hypot(e[k], e[i]);
            }
            if e[k] != 0.0 {
                if e[k + 1] < 0.0 {
                    e[k] = -e[k];
                }
                for i in k + 1..n {
                    e[i] /= e[k];
                }
                e[k + 1] += 1.0;
            }
            e[k] = -e[k];
            if k + 1 < m && e[k] != 0.0 {
                for i in k + 1..m {
                    work[i] = 0.0;
                }
                for j in k + 1..n {
                    for i in k + 1..m {
                        work[i] += e[j] * a[i][j];
                    }
                }
                for j in k + 1..n {
                    let t = -e[j] / e[k + 1];
                    for i in k + 1..m {
                        a[i][j] += t * work[i];
                    }
                }
            }
            for i in k + 1..n {
                v[i][k] = e[i];
            }
        }
    }

    // SVD.js 365-377: the last bidiagonal entries.
    let p0 = n.min(m + 1);
    if nct < n {
        s[nct] = a[nct][nct];
    }
    if m < p0 {
        s[p0 - 1] = 0.0;
    }
    if nrt + 1 < p0 {
        e[nrt] = a[nrt][p0 - 1];
    }
    e[p0 - 1] = 0.0;

    // SVD.js 379-412: the left singular vectors.
    for j in nct..nu {
        for i in 0..m {
            u[i][j] = 0.0;
        }
        u[j][j] = 1.0;
    }
    for k in (0..nct).rev() {
        if s[k] != 0.0 {
            for j in k + 1..nu {
                let mut t = 0.0;
                for i in k..m {
                    t += u[i][k] * u[i][j];
                }
                t = -t / u[k][k];
                for i in k..m {
                    u[i][j] += t * u[i][k];
                }
            }
            for i in k..m {
                u[i][k] = -u[i][k];
            }
            u[k][k] += 1.0;
            // `for (let i = 0; i < k - 1; i++)` (SVD.js 402): no iterations when k is 0 or 1.
            for i in 0..k.saturating_sub(1) {
                u[i][k] = 0.0;
            }
        } else {
            for i in 0..m {
                u[i][k] = 0.0;
            }
            u[k][k] = 1.0;
        }
    }

    // SVD.js 414-433: the right singular vectors.
    for k in (0..n).rev() {
        if k < nrt && e[k] != 0.0 {
            for j in k + 1..nu {
                let mut t = 0.0;
                for i in k + 1..n {
                    t += v[i][k] * v[i][j];
                }
                t = -t / v[k + 1][k];
                for i in k + 1..n {
                    v[i][j] += t * v[i][k];
                }
            }
        }
        for i in 0..n {
            v[i][k] = 0.0;
        }
        v[k][k] = 1.0;
    }

    // SVD.js 435-439: the iteration. `k` is signed: the source's first loop stops at -1.
    let mut p = p0;
    let pp = p - 1;
    let eps = ferrigura_jsmath::pow(2.0, -52.0);
    let tiny = ferrigura_jsmath::pow(2.0, -966.0);
    while p > 0 {
        // SVD.js 442-447: the search for a negligible superdiagonal entry.
        let mut k = p as isize - 2;
        while k >= -1 {
            if k == -1 {
                break;
            }
            let ku = k as usize;
            if e[ku].abs() <= tiny + eps * (s[ku].abs() + s[ku + 1].abs()) {
                e[ku] = 0.0;
                break;
            }
            k -= 1;
        }
        let kase: u8;
        if k == p as isize - 2 {
            kase = 4;
        } else {
            // SVD.js 454-464. `ks !== p` of the source is always true (`ks` starts at `p - 1`).
            let mut ks = p as isize - 1;
            while ks >= k {
                if ks == k {
                    break;
                }
                let ksu = ks as usize;
                let t = e[ksu].abs()
                    + if ks != k + 1 {
                        e[(ks - 1) as usize].abs()
                    } else {
                        0.0
                    };
                if s[ksu].abs() <= tiny + eps * t {
                    s[ksu] = 0.0;
                    break;
                }
                ks -= 1;
            }
            if ks == k {
                kase = 3;
            } else if ks == p as isize - 1 {
                kase = 1;
            } else {
                kase = 2;
                k = ks;
            }
        }
        k += 1;
        let ku = k as usize;
        match kase {
            1 => {
                let pm2 = p - 2;
                let mut f = e[pm2];
                e[pm2] = 0.0;
                for j in (ku..=pm2).rev() {
                    let mut t = svd_hypot(s[j], f);
                    let cs = s[j] / t;
                    let sn = f / t;
                    s[j] = t;
                    if j != ku {
                        f = -sn * e[j - 1];
                        e[j - 1] *= cs;
                    }
                    for i in 0..n {
                        t = cs * v[i][j] + sn * v[i][p - 1];
                        v[i][p - 1] = -sn * v[i][j] + cs * v[i][p - 1];
                        v[i][j] = t;
                    }
                }
            }
            2 => {
                let mut f = e[ku - 1];
                e[ku - 1] = 0.0;
                for j in ku..p {
                    let mut t = svd_hypot(s[j], f);
                    let cs = s[j] / t;
                    let sn = f / t;
                    s[j] = t;
                    f = -sn * e[j];
                    e[j] *= cs;
                    for i in 0..m {
                        t = cs * u[i][j] + sn * u[i][ku - 1];
                        u[i][ku - 1] = -sn * u[i][j] + cs * u[i][ku - 1];
                        u[i][j] = t;
                    }
                }
            }
            3 => {
                let pm1 = p - 1;
                let pm2 = p - 2;
                // SVD.js 522: a nested `Math.max`, which propagates NaN.
                let scale = js_max(
                    js_max(
                        js_max(js_max(s[pm1].abs(), s[pm2].abs()), e[pm2].abs()),
                        s[ku].abs(),
                    ),
                    e[ku].abs(),
                );
                let sp = s[pm1] / scale;
                let spm1 = s[pm2] / scale;
                let epm1 = e[pm2] / scale;
                let sk = s[ku] / scale;
                let ek = e[ku] / scale;
                let b = ((spm1 + sp) * (spm1 - sp) + epm1 * epm1) / 2.0;
                let c = (sp * epm1) * (sp * epm1);
                let mut shift = 0.0;
                if b != 0.0 || c != 0.0 {
                    shift = (b * b + c).sqrt();
                    if b < 0.0 {
                        shift = -shift;
                    }
                    shift = c / (b + shift);
                }
                let mut f = (sk + sp) * (sk - sp) + shift;
                let mut g = sk * ek;
                for j in ku..pm1 {
                    let mut t = svd_hypot(f, g);
                    let mut cs = f / t;
                    let mut sn = g / t;
                    if j != ku {
                        e[j - 1] = t;
                    }
                    f = cs * s[j] + sn * e[j];
                    e[j] = cs * e[j] - sn * s[j];
                    g = sn * s[j + 1];
                    s[j + 1] *= cs;
                    for i in 0..n {
                        t = cs * v[i][j] + sn * v[i][j + 1];
                        v[i][j + 1] = -sn * v[i][j] + cs * v[i][j + 1];
                        v[i][j] = t;
                    }
                    t = svd_hypot(f, g);
                    cs = f / t;
                    sn = g / t;
                    s[j] = t;
                    f = cs * e[j] + sn * s[j + 1];
                    s[j + 1] = -sn * e[j] + cs * s[j + 1];
                    g = sn * e[j + 1];
                    e[j + 1] *= cs;
                    // SVD.js 568: `j < this.m - 1`.
                    if j + 1 < m {
                        for i in 0..m {
                            t = cs * u[i][j] + sn * u[i][j + 1];
                            u[i][j + 1] = -sn * u[i][j] + cs * u[i][j + 1];
                            u[i][j] = t;
                        }
                    }
                }
                e[pm2] = f;
            }
            _ => {
                // SVD.js 582-586: a non-positive singular value is made positive.
                let mut kk = ku;
                if s[kk] <= 0.0 {
                    s[kk] = if s[kk] < 0.0 { -s[kk] } else { 0.0 };
                    for i in 0..=pp {
                        v[i][kk] = -v[i][kk];
                    }
                }
                // SVD.js 587-611: sort the singular values into decreasing order.
                while kk < pp {
                    if s[kk] >= s[kk + 1] {
                        break;
                    }
                    s.swap(kk, kk + 1);
                    // SVD.js 597 and 604: `k < this.n - 1` and `k < this.m - 1`.
                    if kk + 1 < n {
                        for i in 0..n {
                            v[i].swap(kk, kk + 1);
                        }
                    }
                    if kk + 1 < m {
                        for i in 0..m {
                            u[i].swap(kk, kk + 1);
                        }
                    }
                    kk += 1;
                }
                p -= 1;
            }
        }
    }

    Svd { u, v, s }
}

/// The orthogonal Procrustes fit of cose-base 2.2.0's constraint handler. There is no separate
/// `solveOrthogonalProcrustes` in the source: the fit is inline in `handleConstraints`
/// (ConstraintHandler.js 581-593), and the setup above it (384-437) builds the matrices. Input:
/// the target and source configurations, one `[x, y]` row per node (`targetMatrix` and
/// `sourceMatrix`). Output: `T = V U'`, which the caller applies to each coordinate pair
/// (594-601, not ported here).
///
/// Errors: an empty input, where the source throws on `array[0].length` (581); and a non-finite
/// product, where the source's SVD loop does not end (`svd` has no guard in the source).
pub(crate) fn orthogonal_procrustes(
    target: &[[f64; 2]],
    source: &[[f64; 2]],
) -> Result<[[f64; 2]; 2], Error> {
    if target.is_empty() || source.is_empty() {
        return Err(Error::Unsupported(
            "a Procrustes fit with no node: cose-base throws on array[0].length (ConstraintHandler.js 581)",
        ));
    }
    let rows = |configuration: &[[f64; 2]]| -> Vec<Vec<f64>> {
        configuration.iter().map(|row| row.to_vec()).collect()
    };
    // A' and B' (ConstraintHandler.js 581-582), each centred by row (586-587).
    let mut target_transpose = transpose(&rows(target));
    let mut source_transpose = transpose(&rows(source));
    for row in target_transpose.iter_mut() {
        *row = mult_gamma(row);
    }
    for row in source_transpose.iter_mut() {
        *row = mult_gamma(row);
    }
    // tempMatrix = A'B (ConstraintHandler.js 591), then SVD(A'B) = U S V' (592).
    let temp = mult_mat(&target_transpose, &transpose(&source_transpose));
    if temp.iter().flatten().any(|x| !x.is_finite()) {
        return Err(Error::Unsupported(
            "a Procrustes fit with a non-finite coordinate: the SVD of cose-base never ends",
        ));
    }
    let decomposition = svd(temp);
    // transformationMatrix = V U' (ConstraintHandler.js 593).
    let t = mult_mat(&decomposition.v, &transpose(&decomposition.u));
    Ok([[t[0][0], t[0][1]], [t[1][0], t[1][1]]])
}
