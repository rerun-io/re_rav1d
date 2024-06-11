use crate::include::common::bitdepth::BitDepth;
use crate::include::dav1d::headers::Rav1dFrameHeader;
use crate::include::dav1d::headers::Rav1dPixelLayout;
use crate::include::dav1d::picture::Rav1dPictureDataComponentOffset;
use crate::src::align::AlignedVec64;
use crate::src::disjoint_mut::DisjointMut;
use crate::src::internal::Rav1dBitDepthDSPContext;
use crate::src::internal::Rav1dContext;
use crate::src::internal::Rav1dFrameData;
use crate::src::lr_apply::LR_RESTORE_U;
use crate::src::lr_apply::LR_RESTORE_V;
use crate::src::lr_apply::LR_RESTORE_Y;
use crate::src::relaxed_atomic::RelaxedAtomic;
use crate::src::unstable_extensions::as_chunks;
use crate::src::unstable_extensions::flatten;
use libc::ptrdiff_t;
use std::array;
use std::cmp;
use std::ffi::c_int;
use std::ffi::c_uint;

// The loop filter buffer stores 12 rows of pixels. A superblock block will
// contain at most 2 stripes. Each stripe requires 4 rows pixels (2 above
// and 2 below) the final 4 rows are used to swap the bottom of the last
// stripe with the top of the next super block row.
unsafe fn backup_lpf<BD: BitDepth>(
    c: &Rav1dContext,
    dst: &DisjointMut<AlignedVec64<u8>>,
    mut dst_offset: usize, // in pixel units
    dst_stride: ptrdiff_t,
    mut src: Rav1dPictureDataComponentOffset,
    ss_ver: c_int,
    sb128: u8,
    mut row: c_int,
    row_h: c_int,
    src_w: c_int,
    h: c_int,
    ss_hor: c_int,
    lr_backup: c_int,
    frame_hdr: &Rav1dFrameHeader,
    dsp: &Rav1dBitDepthDSPContext,
    resize_step: [c_int; 2],
    resize_start: [c_int; 2],
    bd: BD,
) {
    let src_w = src_w as usize;

    let cdef_backup = (lr_backup == 0) as c_int;
    let dst_w = if frame_hdr.size.super_res.enabled {
        (frame_hdr.size.width[1] + ss_hor) as usize >> ss_hor
    } else {
        src_w
    };
    // The first stripe of the frame is shorter by 8 luma pixel rows.
    let mut stripe_h = (64 << (cdef_backup & sb128 as c_int)) - 8 * (row == 0) as c_int >> ss_ver;
    src += (stripe_h - 2) as isize * src.data.pixel_stride::<BD>();
    if c.tc.len() == 1 {
        if row != 0 {
            let top = 4 << sb128;
            let px_abs_stride = BD::pxstride(dst_stride.unsigned_abs());
            let top_size = top * px_abs_stride;
            // Copy the top part of the stored loop filtered pixels from the
            // previous sb row needed above the first stripe of this sb row.
            let (dst_idx, src_idx) = if dst_stride < 0 {
                (
                    dst_offset - 3 * px_abs_stride,
                    dst_offset - top_size - 3 * px_abs_stride,
                )
            } else {
                (dst_offset, dst_offset + top_size)
            };

            for i in 0..4 {
                BD::pixel_copy(
                    &mut dst.mut_slice_as((dst_idx + i * px_abs_stride.., ..dst_w)),
                    &dst.slice_as((src_idx + i * px_abs_stride.., ..dst_w)),
                    dst_w,
                );
            }
        }
        dst_offset = (dst_offset as isize + 4 * BD::pxstride(dst_stride)) as usize;
    }
    if lr_backup != 0 && frame_hdr.size.width[0] != frame_hdr.size.width[1] {
        while row + stripe_h <= row_h {
            let n_lines = 4 - (row + stripe_h + 1 == h) as c_int;
            let mut dst_guard = dst.mut_slice_as((dst_offset.., ..dst_w));
            dsp.mc.resize.call::<BD>(
                dst_guard.as_mut_ptr(),
                dst_stride,
                src,
                dst_w,
                n_lines as usize,
                src_w,
                resize_step[ss_hor as usize],
                resize_start[ss_hor as usize],
                bd,
            );
            row += stripe_h; // unmodified stripe_h for the 1st stripe
            stripe_h = 64 >> ss_ver;
            src += stripe_h as isize * src.data.pixel_stride::<BD>();
            dst_offset =
                (dst_offset as isize + n_lines as isize * BD::pxstride(dst_stride)) as usize;

            if n_lines == 3 {
                let dst_abs_px_stride = BD::pxstride(dst_stride.unsigned_abs());
                let (src_idx, dst_idx) = if dst_stride < 0 {
                    (dst_offset + dst_abs_px_stride, dst_offset)
                } else {
                    (dst_offset - dst_abs_px_stride, dst_offset)
                };
                BD::pixel_copy(
                    &mut dst.mut_slice_as((dst_idx.., ..dst_w)),
                    &dst.slice_as((src_idx.., ..dst_w)),
                    dst_w,
                );
                dst_offset = (dst_offset as isize + BD::pxstride(dst_stride)) as usize;
            }
        }
    } else {
        while row + stripe_h <= row_h {
            let n_lines = 4 - (row + stripe_h + 1 == h) as c_int;
            for i in 0..4 {
                let dst_abs_px_stride = BD::pxstride(dst_stride.unsigned_abs());
                if i != n_lines {
                    BD::pixel_copy(
                        &mut dst.mut_slice_as((dst_offset.., ..src_w)),
                        &src.data.slice::<BD, _>((src.offset.., ..src_w)),
                        src_w,
                    );
                } else {
                    let (src_idx, dst_idx) = if dst_stride < 0 {
                        (dst_offset + dst_abs_px_stride, dst_offset)
                    } else {
                        (dst_offset - dst_abs_px_stride, dst_offset)
                    };
                    BD::pixel_copy(
                        &mut dst.mut_slice_as((dst_idx.., ..src_w)),
                        &dst.slice_as((src_idx.., ..src_w)),
                        src_w,
                    )
                }
                dst_offset = (dst_offset as isize + BD::pxstride(dst_stride)) as usize;
                src += src.data.pixel_stride::<BD>();
            }
            row += stripe_h; // unmodified stripe_h for the 1st stripe
            stripe_h = 64 >> ss_ver;
            src += (stripe_h - 4) as isize * src.data.pixel_stride::<BD>();
        }
    };
}

pub(crate) unsafe fn rav1d_copy_lpf<BD: BitDepth>(
    c: &Rav1dContext,
    f: &Rav1dFrameData,
    src: [Rav1dPictureDataComponentOffset; 3],
    sby: c_int,
) {
    let bd = BD::from_c(f.bitdepth_max);

    let have_tt = (c.tc.len() > 1) as c_int;
    let frame_hdr = &***f.frame_hdr.as_ref().unwrap();
    let resize = (frame_hdr.size.width[0] != frame_hdr.size.width[1]) as c_int;
    let offset_y = 8 * (sby != 0) as c_int;
    let seq_hdr = &***f.seq_hdr.as_ref().unwrap();
    let tt_off = have_tt * sby * (4 << seq_hdr.sb128);
    let sr_cur_data = &f.sr_cur.p.data.as_ref().unwrap().data;
    let dst = array::from_fn::<_, 3, _>(|i| {
        let data = &sr_cur_data[i];
        let offset =
            f.lf.lr_lpf_line[i].wrapping_add_signed(tt_off as isize * data.pixel_stride::<BD>());
        Rav1dPictureDataComponentOffset { data, offset }
    });

    // TODO Also check block level restore type to reduce copying.
    let restore_planes = f.lf.restore_planes;

    if seq_hdr.cdef != 0 || restore_planes & LR_RESTORE_Y as c_int != 0 {
        let h = f.cur.p.h;
        let w = f.bw << 2;
        let row_h = cmp::min((sby + 1) << 6 + seq_hdr.sb128, h - 1);
        let y_stripe = (sby << 6 + seq_hdr.sb128) - offset_y;
        if restore_planes & LR_RESTORE_Y as c_int != 0 || resize == 0 {
            backup_lpf::<BD>(
                c,
                &f.lf.lr_line_buf,
                dst[0].offset,
                dst[0].data.stride(),
                src[0] - (offset_y as isize * src[0].data.pixel_stride::<BD>()),
                0,
                seq_hdr.sb128,
                y_stripe,
                row_h,
                w,
                h,
                0,
                1,
                frame_hdr,
                f.dsp,
                f.resize_step,
                f.resize_start,
                bd,
            );
        }
        if have_tt != 0 && resize != 0 {
            let cdef_off_y = (sby * 4) as isize * src[0].data.pixel_stride::<BD>();
            let cdef_plane_y_sz = 4 * f.sbh as isize * src[0].data.pixel_stride::<BD>();
            let y_span = cdef_plane_y_sz - src[0].data.pixel_stride::<BD>();
            let cdef_line_start = (f.lf.cdef_lpf_line[0] as isize + cmp::min(y_span, 0)) as usize;
            backup_lpf::<BD>(
                c,
                &f.lf.cdef_line_buf,
                cdef_line_start + (cdef_off_y - cmp::min(y_span, 0)) as usize,
                src[0].data.stride(),
                src[0] - (offset_y as isize * src[0].data.pixel_stride::<BD>()),
                0,
                seq_hdr.sb128,
                y_stripe,
                row_h,
                w,
                h,
                0,
                0,
                frame_hdr,
                f.dsp,
                f.resize_step,
                f.resize_start,
                bd,
            );
        }
    }
    if (seq_hdr.cdef != 0 || restore_planes & (LR_RESTORE_U as c_int | LR_RESTORE_V as c_int) != 0)
        && f.cur.p.layout != Rav1dPixelLayout::I400
    {
        let ss_ver = (f.sr_cur.p.p.layout == Rav1dPixelLayout::I420) as c_int;
        let ss_hor = (f.sr_cur.p.p.layout != Rav1dPixelLayout::I444) as c_int;
        let h_0 = f.cur.p.h + ss_ver >> ss_ver;
        let w_0 = f.bw << 2 - ss_hor;
        let row_h_0 = cmp::min((sby + 1) << 6 - ss_ver + seq_hdr.sb128 as c_int, h_0 - 1);
        let offset_uv = offset_y >> ss_ver;
        let y_stripe_0 = (sby << 6 - ss_ver + seq_hdr.sb128 as c_int) - offset_uv;
        let cdef_off_uv = sby as isize * 4 * src[1].data.pixel_stride::<BD>();
        if seq_hdr.cdef != 0 || restore_planes & LR_RESTORE_U as c_int != 0 {
            if restore_planes & LR_RESTORE_U as c_int != 0 || resize == 0 {
                backup_lpf::<BD>(
                    c,
                    &f.lf.lr_line_buf,
                    dst[1].offset,
                    dst[1].data.stride(),
                    src[1] - (offset_uv as isize * src[1].data.pixel_stride::<BD>()),
                    ss_ver,
                    seq_hdr.sb128,
                    y_stripe_0,
                    row_h_0,
                    w_0,
                    h_0,
                    ss_hor,
                    1,
                    frame_hdr,
                    f.dsp,
                    f.resize_step,
                    f.resize_start,
                    bd,
                );
            }
            if have_tt != 0 && resize != 0 {
                let cdef_plane_uv_sz = 4 * f.sbh as isize * src[1].data.pixel_stride::<BD>();
                let uv_span = cdef_plane_uv_sz - src[1].data.pixel_stride::<BD>();
                let cdef_line_start =
                    (f.lf.cdef_lpf_line[1] as isize + cmp::min(uv_span, 0)) as usize;
                backup_lpf::<BD>(
                    c,
                    &f.lf.cdef_line_buf,
                    cdef_line_start + (cdef_off_uv - cmp::min(uv_span, 0)) as usize,
                    src[1].data.stride(),
                    src[1] - (offset_uv as isize * src[1].data.pixel_stride::<BD>()),
                    ss_ver,
                    seq_hdr.sb128,
                    y_stripe_0,
                    row_h_0,
                    w_0,
                    h_0,
                    ss_hor,
                    0,
                    frame_hdr,
                    f.dsp,
                    f.resize_step,
                    f.resize_start,
                    bd,
                );
            }
        }
        if seq_hdr.cdef != 0 || restore_planes & LR_RESTORE_V as c_int != 0 {
            if restore_planes & LR_RESTORE_V as c_int != 0 || resize == 0 {
                backup_lpf::<BD>(
                    c,
                    &f.lf.lr_line_buf,
                    dst[2].offset,
                    dst[2].data.stride(),
                    src[2] - (offset_uv as isize * src[2].data.pixel_stride::<BD>()),
                    ss_ver,
                    seq_hdr.sb128,
                    y_stripe_0,
                    row_h_0,
                    w_0,
                    h_0,
                    ss_hor,
                    1,
                    frame_hdr,
                    f.dsp,
                    f.resize_step,
                    f.resize_start,
                    bd,
                );
            }
            if have_tt != 0 && resize != 0 {
                let cdef_plane_uv_sz = 4 * f.sbh as isize * src[2].data.pixel_stride::<BD>();
                let uv_span = cdef_plane_uv_sz - src[2].data.pixel_stride::<BD>();
                let cdef_line_start =
                    (f.lf.cdef_lpf_line[2] as isize + cmp::min(uv_span, 0)) as usize;
                backup_lpf::<BD>(
                    c,
                    &f.lf.cdef_line_buf,
                    cdef_line_start + (cdef_off_uv - cmp::min(uv_span, 0)) as usize,
                    src[2].data.stride(),
                    src[2] - (offset_uv as isize * src[2].data.pixel_stride::<BD>()),
                    ss_ver,
                    seq_hdr.sb128,
                    y_stripe_0,
                    row_h_0,
                    w_0,
                    h_0,
                    ss_hor,
                    0,
                    frame_hdr,
                    f.dsp,
                    f.resize_step,
                    f.resize_start,
                    bd,
                );
            }
        }
    }
}

/// Slice `[u8; 4]`s from `lvl`, but "unaligned",
/// meaning the `[u8; 4]`s can straddle
/// adjacent `[u8; 4]`s in the `lvl` slice.
///
/// Note that this does not result in actual unaligned reads,
/// since `[u8; 4]` has an alignment of 1.
/// This optimizes to a single slice with a bounds check.
#[inline(always)]
fn unaligned_lvl_slice(lvl: &[[u8; 4]], y: usize) -> &[[u8; 4]] {
    as_chunks(&flatten(lvl)[y..]).0
}

#[inline]
unsafe fn filter_plane_cols_y<BD: BitDepth>(
    f: &Rav1dFrameData,
    have_left: bool,
    lvl: &[[u8; 4]],
    mask: &[[[RelaxedAtomic<u16>; 2]; 3]; 32],
    y_dst: Rav1dPictureDataComponentOffset,
    w: usize,
    starty4: usize,
    endy4: usize,
) {
    // filter edges between columns (e.g. block1 | block2)
    let mask = &mask[..w];
    for x in 0..w {
        if !have_left && x == 0 {
            continue;
        }
        let mask = &mask[x];
        let hmask = if starty4 == 0 {
            if endy4 > 16 {
                mask.each_ref()
                    .map(|[a, b]| a.get() as u32 | ((b.get() as u32) << 16))
            } else {
                mask.each_ref().map(|[a, _]| a.get() as u32)
            }
        } else {
            mask.each_ref().map(|[_, b]| b.get() as u32)
        };
        f.dsp.lf.loop_filter_sb.y.h.call::<BD>(
            f,
            y_dst + x * 4,
            &hmask,
            &lvl[x..],
            endy4 - starty4,
        );
    }
}

#[inline]
unsafe fn filter_plane_rows_y<BD: BitDepth>(
    f: &Rav1dFrameData,
    have_top: bool,
    lvl: &[[u8; 4]],
    b4_stride: usize,
    mask: &[[[RelaxedAtomic<u16>; 2]; 3]; 32],
    y_dst: Rav1dPictureDataComponentOffset,
    w: usize,
    starty4: usize,
    endy4: usize,
) {
    //                                 block1
    // filter edges between rows (e.g. ------)
    //                                 block2
    let len = endy4 - starty4;
    for i in 0..len {
        let y = i + starty4;
        let y_dst = y_dst + (i as isize * 4 * y_dst.data.pixel_stride::<BD>());
        if !have_top && y == 0 {
            continue;
        }
        let mask = &mask[y];
        let vmask = mask
            .each_ref()
            .map(|[a, b]| a.get() as u32 | ((b.get() as u32) << 16));
        let lvl = &lvl[i * b4_stride..];
        f.dsp
            .lf
            .loop_filter_sb
            .y
            .v
            .call::<BD>(f, y_dst, &vmask, unaligned_lvl_slice(lvl, 1), w);
    }
}

#[inline]
unsafe fn filter_plane_cols_uv<BD: BitDepth>(
    f: &Rav1dFrameData,
    have_left: bool,
    lvl: &[[u8; 4]],
    mask: &[[[RelaxedAtomic<u16>; 2]; 2]; 32],
    u_dst: Rav1dPictureDataComponentOffset,
    v_dst: Rav1dPictureDataComponentOffset,
    w: usize,
    starty4: usize,
    endy4: usize,
    ss_ver: c_int,
) {
    // filter edges between columns (e.g. block1 | block2)
    let mask = &mask[..w];
    let lvl = &lvl[..w];
    for x in 0..w {
        if !have_left && x == 0 {
            continue;
        }
        let mask = &mask[x];
        let hmask = if starty4 == 0 {
            if endy4 > 16 >> ss_ver {
                mask.each_ref()
                    .map(|[a, b]| a.get() as u32 | ((b.get() as u32) << (16 >> ss_ver)))
            } else {
                mask.each_ref().map(|[a, _]| a.get() as u32)
            }
        } else {
            mask.each_ref().map(|[_, b]| b.get() as u32)
        };
        let hmask = [hmask[0], hmask[1], 0];
        let lvl = &lvl[x..];
        f.dsp.lf.loop_filter_sb.uv.h.call::<BD>(
            f,
            u_dst + x * 4,
            &hmask,
            unaligned_lvl_slice(lvl, 2),
            endy4 - starty4,
        );
        f.dsp.lf.loop_filter_sb.uv.h.call::<BD>(
            f,
            v_dst + x * 4,
            &hmask,
            unaligned_lvl_slice(lvl, 3),
            endy4 - starty4,
        );
    }
}

#[inline]
unsafe fn filter_plane_rows_uv<BD: BitDepth>(
    f: &Rav1dFrameData,
    have_top: bool,
    lvl: &[[u8; 4]],
    b4_stride: usize,
    mask: &[[[RelaxedAtomic<u16>; 2]; 2]; 32],
    u_dst: Rav1dPictureDataComponentOffset,
    v_dst: Rav1dPictureDataComponentOffset,
    w: usize,
    starty4: usize,
    endy4: usize,
    ss_hor: c_int,
) {
    //                                 block1
    // filter edges between rows (e.g. ------)
    //                                 block2
    let len = endy4 - starty4;
    for i in 0..len {
        let y = i + starty4;
        let u_dst = u_dst + (i as isize * 4 * u_dst.data.pixel_stride::<BD>());
        let v_dst = v_dst + (i as isize * 4 * v_dst.data.pixel_stride::<BD>());
        if !have_top && y == 0 {
            continue;
        }
        let vmask = mask[y]
            .each_ref()
            .map(|[a, b]| a.get() as u32 | ((b.get() as u32) << (16 >> ss_hor)));
        let vmask = [vmask[0], vmask[1], 0];
        let lvl = &lvl[i * b4_stride..];
        f.dsp
            .lf
            .loop_filter_sb
            .uv
            .v
            .call::<BD>(f, u_dst, &vmask, unaligned_lvl_slice(lvl, 2), w);
        f.dsp
            .lf
            .loop_filter_sb
            .uv
            .v
            .call::<BD>(f, v_dst, &vmask, unaligned_lvl_slice(lvl, 3), w);
    }
}

pub(crate) unsafe fn rav1d_loopfilter_sbrow_cols<BD: BitDepth>(
    f: &Rav1dFrameData,
    [py, pu, pv]: [Rav1dPictureDataComponentOffset; 3],
    lflvl_offset: usize,
    sby: c_int,
    start_of_tile_row: c_int,
) {
    let lflvl = &f.lf.mask[lflvl_offset..];
    let mut have_left; // Don't filter outside the frame
    let seq_hdr = &***f.seq_hdr.as_ref().unwrap();
    let is_sb64 = (seq_hdr.sb128 == 0) as c_int;
    let starty4 = ((sby & is_sb64) as u32) << 4;
    let sbsz = 32 >> is_sb64;
    let sbl2 = 5 - is_sb64;
    let halign = (f.bh + 31 & !31) as usize;
    let ss_ver = (f.cur.p.layout == Rav1dPixelLayout::I420) as c_int;
    let ss_hor = (f.cur.p.layout != Rav1dPixelLayout::I444) as c_int;
    let vmask = 16 >> ss_ver;
    let hmask = 16 >> ss_hor;
    let vmax = (1 as c_uint) << vmask;
    let hmax = (1 as c_uint) << hmask;
    let endy4 = starty4 + cmp::min(f.h4 - sby * sbsz, sbsz) as u32;
    let uv_endy4 = (endy4 + ss_ver as u32) >> ss_ver;
    let mut lpf_y_idx = (sby << sbl2) as usize;
    let mut lpf_uv_idx = (sby << sbl2 - ss_ver) as usize;
    let frame_hdr = &***f.frame_hdr.as_ref().unwrap();

    // fix lpf strength at tile col boundaries
    let mut tile_col = 1;
    loop {
        let mut x = frame_hdr.tiling.col_start_sb[tile_col as usize] as c_int;
        if x << sbl2 >= f.bw {
            break;
        }
        let bx4: c_int = if x & is_sb64 != 0 { 16 } else { 0 };
        let cbx4 = bx4 >> ss_hor;
        x >>= is_sb64;
        let y_hmask = &lflvl[x as usize].filter_y[0][bx4 as usize];
        let (lpf_y, lpf_uv) = f.lf.tx_lpf_right_edge.get(
            lpf_y_idx..lpf_y_idx + (endy4 - starty4) as usize,
            lpf_uv_idx..lpf_uv_idx + (uv_endy4 - (starty4 >> ss_ver)) as usize,
        );
        for y in starty4..endy4 {
            let mask: u32 = 1 << y;
            let sidx = (mask >= 0x10000) as usize;
            let smask = (mask >> (sidx << 4)) as u16;
            let idx = 2 * (y_hmask[2][sidx].get() & smask != 0) as usize
                + (y_hmask[1][sidx].get() & smask != 0) as usize;
            y_hmask[2][sidx].update(|it| it & !smask);
            y_hmask[1][sidx].update(|it| it & !smask);
            y_hmask[0][sidx].update(|it| it & !smask);
            y_hmask[cmp::min(idx, lpf_y[(y - starty4) as usize] as usize)][sidx]
                .update(|it| it | smask);
        }
        if f.cur.p.layout != Rav1dPixelLayout::I400 {
            let uv_hmask = &lflvl[x as usize].filter_uv[0][cbx4 as usize];
            for y in starty4 >> ss_ver..uv_endy4 {
                let uv_mask: u32 = 1 << y;
                let sidx = (uv_mask >= vmax) as usize;
                let smask = (uv_mask >> (sidx << 4 - ss_ver)) as u16;
                let idx = (uv_hmask[1][sidx].get() & smask != 0) as usize;
                uv_hmask[1][sidx].update(|it| it & !smask);
                uv_hmask[0][sidx].update(|it| it & !smask);
                uv_hmask[cmp::min(idx, lpf_uv[(y - (starty4 >> ss_ver)) as usize] as usize)][sidx]
                    .update(|it| it | smask);
            }
        }
        lpf_y_idx += halign;
        lpf_uv_idx += halign >> ss_ver;
        tile_col += 1;
    }

    // fix lpf strength at tile row boundaries
    if start_of_tile_row != 0 {
        let mut a = &f.a[(f.sb128w * (start_of_tile_row - 1)) as usize..];
        for x in 0..f.sb128w {
            let y_vmask = &lflvl[x as usize].filter_y[1][starty4 as usize];
            let w = cmp::min(32, f.w4 - (x << 5)) as u32;
            for i in 0..w {
                let mask: u32 = 1 << i;
                let sidx = (mask >= 0x10000) as usize;
                let smask = (mask >> (sidx << 4)) as u16;
                let idx = 2 * (y_vmask[2][sidx].get() & smask != 0) as usize
                    + (y_vmask[1][sidx].get() & smask != 0) as usize;
                y_vmask[2][sidx].update(|it| it & !smask);
                y_vmask[1][sidx].update(|it| it & !smask);
                y_vmask[0][sidx].update(|it| it & !smask);
                y_vmask[cmp::min(idx, *a[0].tx_lpf_y.index(i as usize) as usize)][sidx]
                    .update(|it| it | smask);
            }
            if f.cur.p.layout != Rav1dPixelLayout::I400 {
                let cw: c_uint = w.wrapping_add(ss_hor as c_uint) >> ss_hor;
                let uv_vmask = &lflvl[x as usize].filter_uv[1][(starty4 >> ss_ver) as usize];
                for i in 0..cw {
                    let uv_mask: u32 = 1 << i;
                    let sidx = (uv_mask >= hmax) as usize;
                    let smask = (uv_mask >> (sidx << 4 - ss_hor)) as u16;
                    let idx = (uv_vmask[1][sidx].get() & smask != 0) as usize;
                    uv_vmask[1][sidx].update(|it| it & !smask);
                    uv_vmask[0][sidx].update(|it| it & !smask);
                    uv_vmask[cmp::min(idx, *a[0].tx_lpf_uv.index(i as usize) as usize)][sidx]
                        .update(|it| it | smask);
                }
            }
            a = &a[1..];
        }
    }
    let lflvl = &f.lf.mask[lflvl_offset..];
    let level_ptr_guard =
        f.lf.level
            .index((f.b4_stride * sby as isize * sbsz as isize) as usize..);
    let mut level_ptr = &*level_ptr_guard;
    have_left = false;
    for x in 0..f.sb128w as usize {
        filter_plane_cols_y::<BD>(
            f,
            have_left,
            level_ptr,
            &lflvl[x].filter_y[0],
            py + x * 128,
            cmp::min(32, f.w4 - x as c_int * 32) as usize,
            starty4 as usize,
            endy4 as usize,
        );
        have_left = true;
        level_ptr = &level_ptr[32..];
    }
    if frame_hdr.loopfilter.level_u == 0 && frame_hdr.loopfilter.level_v == 0 {
        return;
    }
    let level_ptr_guard =
        f.lf.level
            .index((f.b4_stride * (sby * sbsz >> ss_ver) as isize) as usize..);
    let mut level_ptr = &*level_ptr_guard;
    have_left = false;
    for x in 0..f.sb128w as usize {
        filter_plane_cols_uv::<BD>(
            f,
            have_left,
            level_ptr,
            &lflvl[x].filter_uv[0],
            pu + x * (128 >> ss_hor),
            pv + x * (128 >> ss_hor),
            (cmp::min(32, f.w4 - x as c_int * 32) + ss_hor >> ss_hor) as usize,
            starty4 as usize >> ss_ver,
            uv_endy4 as usize,
            ss_ver,
        );
        have_left = true;
        level_ptr = &level_ptr[32 >> ss_hor..];
    }
}

pub(crate) unsafe fn rav1d_loopfilter_sbrow_rows<BD: BitDepth>(
    f: &Rav1dFrameData,
    p: [Rav1dPictureDataComponentOffset; 3],
    lflvl_offset: usize,
    sby: c_int,
) {
    let lflvl = &f.lf.mask[lflvl_offset..];

    // Don't filter outside the frame
    let have_top = sby > 0;
    let seq_hdr = &***f.seq_hdr.as_ref().unwrap();
    let is_sb64 = (seq_hdr.sb128 == 0) as c_int;
    let starty4 = (sby & is_sb64) << 4;
    let sbsz = 32 >> is_sb64;
    let ss_ver = (f.cur.p.layout == Rav1dPixelLayout::I420) as c_int;
    let ss_hor = (f.cur.p.layout != Rav1dPixelLayout::I444) as c_int;
    let endy4: c_uint = (starty4 + cmp::min(f.h4 - sby * sbsz, sbsz)) as c_uint;
    let uv_endy4: c_uint = endy4.wrapping_add(ss_ver as c_uint) >> ss_ver;

    let level_ptr_guard =
        f.lf.level
            .index((f.b4_stride * sby as isize * sbsz as isize) as usize..);
    let mut level_ptr = &*level_ptr_guard;
    for x in 0..f.sb128w as usize {
        filter_plane_rows_y::<BD>(
            f,
            have_top,
            level_ptr,
            f.b4_stride as usize,
            &lflvl[x].filter_y[1],
            p[0] + 128 * x,
            cmp::min(32, f.w4 - x as c_int * 32) as usize,
            starty4 as usize,
            endy4 as usize,
        );
        level_ptr = &level_ptr[32..];
    }

    let frame_hdr = &***f.frame_hdr.as_ref().unwrap();
    if frame_hdr.loopfilter.level_u == 0 && frame_hdr.loopfilter.level_v == 0 {
        return;
    }

    let level_ptr_guard =
        f.lf.level
            .index((f.b4_stride * (sby * sbsz >> ss_ver) as isize) as usize..);
    let mut level_ptr = &*level_ptr_guard;
    let [_, pu, pv] = p;
    for x in 0..f.sb128w as usize {
        filter_plane_rows_uv::<BD>(
            f,
            have_top,
            level_ptr,
            f.b4_stride as usize,
            &lflvl[x].filter_uv[1],
            pu + (x * 128 >> ss_hor),
            pv + (x * 128 >> ss_hor),
            (cmp::min(32 as c_int, f.w4 - x as c_int * 32) + ss_hor >> ss_hor) as usize,
            starty4 as usize >> ss_ver,
            uv_endy4 as usize,
            ss_hor,
        );
        level_ptr = &level_ptr[32 >> ss_hor..];
    }
}
