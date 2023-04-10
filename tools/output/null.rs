use crate::include::stddef::*;
use crate::include::stdint::*;
use ::libc;
extern "C" {
    pub type Dav1dRef;
    pub type MuxerPriv;
    fn dav1d_picture_unref(p: *mut Dav1dPicture);
}


use crate::include::dav1d::common::Dav1dDataProps;
use crate::include::dav1d::headers::Dav1dTxfmMode;




use crate::include::dav1d::headers::Dav1dFilterMode;





















use crate::include::dav1d::headers::Dav1dWarpedMotionParams;







use crate::include::dav1d::headers::Dav1dFrameType;

























































use crate::include::dav1d::headers::Dav1dContentLightLevel;
use crate::include::dav1d::headers::Dav1dMasteringDisplay;
use crate::include::dav1d::headers::Dav1dITUTT35;
use crate::include::dav1d::headers::Dav1dSequenceHeader;






#[derive(Copy, Clone)]
#[repr(C)]
pub struct Dav1dFrameHeader {
    pub film_grain: Dav1dFrameHeader_film_grain,
    pub frame_type: Dav1dFrameType,
    pub width: [libc::c_int; 2],
    pub height: libc::c_int,
    pub frame_offset: libc::c_int,
    pub temporal_id: libc::c_int,
    pub spatial_id: libc::c_int,
    pub show_existing_frame: libc::c_int,
    pub existing_frame_idx: libc::c_int,
    pub frame_id: libc::c_int,
    pub frame_presentation_delay: libc::c_int,
    pub show_frame: libc::c_int,
    pub showable_frame: libc::c_int,
    pub error_resilient_mode: libc::c_int,
    pub disable_cdf_update: libc::c_int,
    pub allow_screen_content_tools: libc::c_int,
    pub force_integer_mv: libc::c_int,
    pub frame_size_override: libc::c_int,
    pub primary_ref_frame: libc::c_int,
    pub buffer_removal_time_present: libc::c_int,
    pub operating_points: [Dav1dFrameHeaderOperatingPoint; 32],
    pub refresh_frame_flags: libc::c_int,
    pub render_width: libc::c_int,
    pub render_height: libc::c_int,
    pub super_res: Dav1dFrameHeader_super_res,
    pub have_render_size: libc::c_int,
    pub allow_intrabc: libc::c_int,
    pub frame_ref_short_signaling: libc::c_int,
    pub refidx: [libc::c_int; 7],
    pub hp: libc::c_int,
    pub subpel_filter_mode: Dav1dFilterMode,
    pub switchable_motion_mode: libc::c_int,
    pub use_ref_frame_mvs: libc::c_int,
    pub refresh_context: libc::c_int,
    pub tiling: Dav1dFrameHeader_tiling,
    pub quant: Dav1dFrameHeader_quant,
    pub segmentation: Dav1dFrameHeader_segmentation,
    pub delta: Dav1dFrameHeader_delta,
    pub all_lossless: libc::c_int,
    pub loopfilter: Dav1dFrameHeader_loopfilter,
    pub cdef: Dav1dFrameHeader_cdef,
    pub restoration: Dav1dFrameHeader_restoration,
    pub txfm_mode: Dav1dTxfmMode,
    pub switchable_comp_refs: libc::c_int,
    pub skip_mode_allowed: libc::c_int,
    pub skip_mode_enabled: libc::c_int,
    pub skip_mode_refs: [libc::c_int; 2],
    pub warp_motion: libc::c_int,
    pub reduced_txtp_set: libc::c_int,
    pub gmv: [Dav1dWarpedMotionParams; 7],
}
use crate::include::dav1d::headers::Dav1dFrameHeader_restoration;
use crate::include::dav1d::headers::Dav1dFrameHeader_cdef;
use crate::include::dav1d::headers::Dav1dFrameHeader_loopfilter;
use crate::include::dav1d::headers::Dav1dFrameHeader_delta;


use crate::include::dav1d::headers::Dav1dFrameHeader_segmentation;
use crate::include::dav1d::headers::Dav1dFrameHeader_quant;
use crate::include::dav1d::headers::Dav1dFrameHeader_tiling;
use crate::include::dav1d::headers::Dav1dFrameHeader_super_res;
use crate::include::dav1d::headers::Dav1dFrameHeaderOperatingPoint;
use crate::include::dav1d::headers::Dav1dFrameHeader_film_grain;
use crate::include::dav1d::picture::Dav1dPictureParameters;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Dav1dPicture {
    pub seq_hdr: *mut Dav1dSequenceHeader,
    pub frame_hdr: *mut Dav1dFrameHeader,
    pub data: [*mut libc::c_void; 3],
    pub stride: [ptrdiff_t; 2],
    pub p: Dav1dPictureParameters,
    pub m: Dav1dDataProps,
    pub content_light: *mut Dav1dContentLightLevel,
    pub mastering_display: *mut Dav1dMasteringDisplay,
    pub itut_t35: *mut Dav1dITUTT35,
    pub reserved: [uintptr_t; 4],
    pub frame_hdr_ref: *mut Dav1dRef,
    pub seq_hdr_ref: *mut Dav1dRef,
    pub content_light_ref: *mut Dav1dRef,
    pub mastering_display_ref: *mut Dav1dRef,
    pub itut_t35_ref: *mut Dav1dRef,
    pub reserved_ref: [uintptr_t; 4],
    pub ref_0: *mut Dav1dRef,
    pub allocator_data: *mut libc::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Muxer {
    pub priv_data_size: libc::c_int,
    pub name: *const libc::c_char,
    pub extension: *const libc::c_char,
    pub write_header: Option::<
        unsafe extern "C" fn(
            *mut MuxerPriv,
            *const libc::c_char,
            *const Dav1dPictureParameters,
            *const libc::c_uint,
        ) -> libc::c_int,
    >,
    pub write_picture: Option::<
        unsafe extern "C" fn(*mut MuxerPriv, *mut Dav1dPicture) -> libc::c_int,
    >,
    pub write_trailer: Option::<unsafe extern "C" fn(*mut MuxerPriv) -> ()>,
    pub verify: Option::<
        unsafe extern "C" fn(*mut MuxerPriv, *const libc::c_char) -> libc::c_int,
    >,
}
pub type NullOutputContext = MuxerPriv;
unsafe extern "C" fn null_write(
    _c: *mut NullOutputContext,
    p: *mut Dav1dPicture,
) -> libc::c_int {
    dav1d_picture_unref(p);
    return 0 as libc::c_int;
}
#[no_mangle]
pub static mut null_muxer: Muxer = {
    let mut init = Muxer {
        priv_data_size: 0 as libc::c_int,
        name: b"null\0" as *const u8 as *const libc::c_char,
        extension: b"null\0" as *const u8 as *const libc::c_char,
        write_header: None,
        write_picture: Some(
            null_write
                as unsafe extern "C" fn(
                    *mut NullOutputContext,
                    *mut Dav1dPicture,
                ) -> libc::c_int,
        ),
        write_trailer: None,
        verify: None,
    };
    init
};
