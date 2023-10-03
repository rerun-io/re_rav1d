use std::ffi::c_int;
use strum::FromRepr;

#[derive(Clone, Copy, PartialEq, Eq, FromRepr)]
#[non_exhaustive]
pub enum Rav1dError {
    /// Not actually used (yet), but this forces `0` to be the niche,
    /// which is more optimal since `0` is no error for [`Dav1dResult`].
    _EPERM = 1,

    ENOENT = 2,
    EIO = 5,
    EAGAIN = 11,
    ENOMEM = 12,
    EINVAL = 22,
    ERANGE = 34,
    ENOPROTOOPT = 92,
}

pub type Rav1dResult<T = ()> = Result<T, Rav1dError>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Dav1dResult(pub c_int);

impl From<Rav1dResult> for Dav1dResult {
    #[inline]
    fn from(value: Rav1dResult) -> Self {
        // Doing the `-` negation on both branches
        // makes the code short and branchless.
        Dav1dResult(
            -(match value {
                Ok(()) => 0,
                Err(e) => e as u8 as c_int,
            }),
        )
    }
}

impl TryFrom<Dav1dResult> for Rav1dResult {
    type Error = Dav1dResult;

    #[inline]
    fn try_from(value: Dav1dResult) -> Result<Self, Self::Error> {
        match value.0 {
            0 => Ok(Ok(())),
            e => {
                let e = (-e).try_into().map_err(|_| value)?;
                let e = Rav1dError::from_repr(e).ok_or(value)?;
                Ok(Err(e))
            }
        }
    }
}
