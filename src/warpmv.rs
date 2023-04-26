use crate::include::dav1d::headers::Dav1dWarpedMotionParams;
use crate::include::stdint::*;
use crate::src::levels::mv;
use ::libc;

use crate::include::common::intops::apply_sign;
use crate::include::common::intops::apply_sign64;
use crate::include::common::intops::iclip;
use crate::include::common::intops::u64log2;
use crate::include::common::intops::ulog2;

static div_lut: [u16; 257] = [
    16384, 16320, 16257, 16194, 16132, 16070, 16009, 15948, 15888, 15828, 15768, 15709, 15650,
    15592, 15534, 15477, 15420, 15364, 15308, 15252, 15197, 15142, 15087, 15033, 14980, 14926,
    14873, 14821, 14769, 14717, 14665, 14614, 14564, 14513, 14463, 14413, 14364, 14315, 14266,
    14218, 14170, 14122, 14075, 14028, 13981, 13935, 13888, 13843, 13797, 13752, 13707, 13662,
    13618, 13574, 13530, 13487, 13443, 13400, 13358, 13315, 13273, 13231, 13190, 13148, 13107,
    13066, 13026, 12985, 12945, 12906, 12866, 12827, 12788, 12749, 12710, 12672, 12633, 12596,
    12558, 12520, 12483, 12446, 12409, 12373, 12336, 12300, 12264, 12228, 12193, 12157, 12122,
    12087, 12053, 12018, 11984, 11950, 11916, 11882, 11848, 11815, 11782, 11749, 11716, 11683,
    11651, 11619, 11586, 11555, 11523, 11491, 11460, 11429, 11398, 11367, 11336, 11305, 11275,
    11245, 11215, 11185, 11155, 11125, 11096, 11067, 11038, 11009, 10980, 10951, 10923, 10894,
    10866, 10838, 10810, 10782, 10755, 10727, 10700, 10673, 10645, 10618, 10592, 10565, 10538,
    10512, 10486, 10460, 10434, 10408, 10382, 10356, 10331, 10305, 10280, 10255, 10230, 10205,
    10180, 10156, 10131, 10107, 10082, 10058, 10034, 10010, 9986, 9963, 9939, 9916, 9892, 9869,
    9846, 9823, 9800, 9777, 9754, 9732, 9709, 9687, 9664, 9642, 9620, 9598, 9576, 9554, 9533, 9511,
    9489, 9468, 9447, 9425, 9404, 9383, 9362, 9341, 9321, 9300, 9279, 9259, 9239, 9218, 9198, 9178,
    9158, 9138, 9118, 9098, 9079, 9059, 9039, 9020, 9001, 8981, 8962, 8943, 8924, 8905, 8886, 8867,
    8849, 8830, 8812, 8793, 8775, 8756, 8738, 8720, 8702, 8684, 8666, 8648, 8630, 8613, 8595, 8577,
    8560, 8542, 8525, 8508, 8490, 8473, 8456, 8439, 8422, 8405, 8389, 8372, 8355, 8339, 8322, 8306,
    8289, 8273, 8257, 8240, 8224, 8208, 8192,
];

#[inline]
fn iclip_wmp(v: libc::c_int) -> libc::c_int {
    let cv = iclip(v, i16::MIN.into(), i16::MAX.into());
    apply_sign(cv.abs() + 32 >> 6, cv) * (1 << 6)
}

#[inline]
fn resolve_divisor_32(d: u32) -> (libc::c_int, libc::c_int) {
    let shift = ulog2(d);
    let e = d - (1 << shift);
    let f = if shift > 8 {
        e + (1 << shift - 9) >> shift - 8
    } else {
        e << 8 - shift
    };
    // Use f as lookup into the precomputed table of multipliers
    (shift + 14, div_lut[f as usize] as libc::c_int)
}

pub fn dav1d_get_shear_params(wm: &mut Dav1dWarpedMotionParams) -> bool {
    let mat = &wm.matrix;

    if mat[2] <= 0 {
        return true;
    }

    let alpha = iclip_wmp(mat[2] - 0x10000) as i16;
    let beta = iclip_wmp(mat[3]) as i16;

    let (mut shift, y) = resolve_divisor_32((mat[2]).abs() as u32);
    let y = apply_sign(y, mat[2]);
    let v1 = mat[4] as i64 * 0x10000 * y as i64;
    let rnd = 1 << shift >> 1;
    let gamma = iclip_wmp(apply_sign64(
        (v1.abs() + rnd as i64 >> shift) as libc::c_int,
        v1,
    )) as i16;
    let v2 = mat[3] as i64 * mat[4] as i64 * y as i64;
    let delta = iclip_wmp(
        mat[5] - apply_sign64((v2.abs() + rnd as i64 >> shift) as libc::c_int, v2) - 0x10000,
    ) as i16;
    wm.abcd = [alpha, beta, gamma, delta];

    4 * (alpha as i32).abs() + 7 * (beta as i32).abs() >= 0x10000
        || 4 * (gamma as i32).abs() + 4 * (delta as i32).abs() >= 0x10000
}

fn resolve_divisor_64(d: u64) -> (libc::c_int, libc::c_int) {
    let shift = u64log2(d);
    let e = d - (1 << shift);
    let f = if shift > 8 {
        e + (1 << shift - 9) >> shift - 8
    } else {
        e << 8 - shift
    };
    // Use f as lookup into the precomputed table of multipliers
    (shift + 14, div_lut[f as usize] as libc::c_int)
}

fn get_mult_shift_ndiag(px: i64, idet: libc::c_int, shift: libc::c_int) -> libc::c_int {
    let v1 = px * idet as i64;
    let v2 = apply_sign64((v1.abs() + (1 << shift >> 1) >> shift) as libc::c_int, v1);
    iclip(v2, -0x1fff, 0x1fff)
}

fn get_mult_shift_diag(px: i64, idet: libc::c_int, shift: libc::c_int) -> libc::c_int {
    let v1 = px * idet as i64;
    let v2 = apply_sign64((v1.abs() + (1 << shift >> 1) >> shift) as libc::c_int, v1);
    iclip(v2, 0xe001, 0x11fff)
}

pub unsafe fn dav1d_set_affine_mv2d(
    bw4: libc::c_int,
    bh4: libc::c_int,
    mv: mv,
    wm: *mut Dav1dWarpedMotionParams,
    bx4: libc::c_int,
    by4: libc::c_int,
) {
    let mat = ((*wm).matrix).as_mut_ptr();
    let rsuy = 2 * bh4 - 1;
    let rsux = 2 * bw4 - 1;
    let isuy = by4 * 4 + rsuy;
    let isux = bx4 * 4 + rsux;
    *mat.offset(0) = iclip(
        mv.x as libc::c_int * 0x2000 - (isux * (*mat.offset(2) - 0x10000) + isuy * *mat.offset(3)),
        -0x800000,
        0x7fffff,
    );
    *mat.offset(1) = iclip(
        mv.y as libc::c_int * 0x2000 - (isux * *mat.offset(4) + isuy * (*mat.offset(5) - 0x10000)),
        -0x800000,
        0x7fffff,
    );
}

#[no_mangle]
pub unsafe extern "C" fn dav1d_find_affine_int(
    mut pts: *const [[libc::c_int; 2]; 2],
    np: libc::c_int,
    bw4: libc::c_int,
    bh4: libc::c_int,
    mv: mv,
    wm: *mut Dav1dWarpedMotionParams,
    bx4: libc::c_int,
    by4: libc::c_int,
) -> libc::c_int {
    let mat: *mut int32_t = ((*wm).matrix).as_mut_ptr();
    let mut a: [[libc::c_int; 2]; 2] = [
        [0 as libc::c_int, 0 as libc::c_int],
        [0 as libc::c_int, 0 as libc::c_int],
    ];
    let mut bx: [libc::c_int; 2] = [0 as libc::c_int, 0 as libc::c_int];
    let mut by: [libc::c_int; 2] = [0 as libc::c_int, 0 as libc::c_int];
    let rsuy = 2 * bh4 - 1;
    let rsux = 2 * bw4 - 1;
    let suy = rsuy * 8;
    let sux = rsux * 8;
    let duy = suy + mv.y as libc::c_int;
    let dux = sux + mv.x as libc::c_int;
    let isuy = by4 * 4 + rsuy;
    let isux = bx4 * 4 + rsux;
    let mut i = 0;
    while i < np {
        let dx = (*pts.offset(i as isize))[1][0] - dux;
        let dy = (*pts.offset(i as isize))[1][1] - duy;
        let sx = (*pts.offset(i as isize))[0][0] - sux;
        let sy = (*pts.offset(i as isize))[0][1] - suy;
        if (sx - dx).abs() < 256 && (sy - dy).abs() < 256 {
            a[0][0] += (sx * sx >> 2) + sx * 2 + 8;
            a[0][1] += (sx * sy >> 2) + sx + sy + 4;
            a[1][1] += (sy * sy >> 2) + sy * 2 + 8;
            bx[0] += (sx * dx >> 2) + sx + dx + 8;
            bx[1] += (sy * dx >> 2) + sy + dx + 4;
            by[0] += (sx * dy >> 2) + sx + dy + 4;
            by[1] += (sy * dy >> 2) + sy + dy + 8;
        }
        i += 1;
    }
    let det: int64_t =
        a[0][0] as int64_t * a[1][1] as int64_t - a[0][1] as int64_t * a[0][1] as int64_t;
    if det == 0 {
        return 1 as libc::c_int;
    }
    let (mut shift, idet) = resolve_divisor_64((det as libc::c_longlong).abs() as u64);
    let mut idet = apply_sign64(idet, det);
    shift -= 16 as libc::c_int;
    if shift < 0 {
        idet <<= -shift;
        shift = 0 as libc::c_int;
    }
    *mat.offset(2) = get_mult_shift_diag(
        a[1][1] as int64_t * bx[0] as int64_t - a[0][1] as int64_t * bx[1] as int64_t,
        idet,
        shift,
    );
    *mat.offset(3) = get_mult_shift_ndiag(
        a[0][0] as int64_t * bx[1] as int64_t - a[0][1] as int64_t * bx[0] as int64_t,
        idet,
        shift,
    );
    *mat.offset(4) = get_mult_shift_ndiag(
        a[1][1] as int64_t * by[0] as int64_t - a[0][1] as int64_t * by[1] as int64_t,
        idet,
        shift,
    );
    *mat.offset(5) = get_mult_shift_diag(
        a[0][0] as int64_t * by[1] as int64_t - a[0][1] as int64_t * by[0] as int64_t,
        idet,
        shift,
    );
    *mat.offset(0) = iclip(
        mv.x as libc::c_int * 0x2000 as libc::c_int
            - (isux * (*mat.offset(2) - 0x10000 as libc::c_int) + isuy * *mat.offset(3)),
        -(0x800000 as libc::c_int),
        0x7fffff as libc::c_int,
    );
    *mat.offset(1) = iclip(
        mv.y as libc::c_int * 0x2000 as libc::c_int
            - (isux * *mat.offset(4) + isuy * (*mat.offset(5) - 0x10000 as libc::c_int)),
        -(0x800000 as libc::c_int),
        0x7fffff as libc::c_int,
    );
    return 0 as libc::c_int;
}
