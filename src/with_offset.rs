use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Sub;
use std::ops::SubAssign;

#[derive(Clone, Copy)]
pub struct WithOffset<T> {
    pub data: T,
    pub offset: usize,
}

impl<T> AddAssign<usize> for WithOffset<T> {
    #[cfg_attr(debug_assertions, track_caller)]
    fn add_assign(&mut self, rhs: usize) {
        self.offset += rhs;
    }
}

impl<T> SubAssign<usize> for WithOffset<T> {
    #[cfg_attr(debug_assertions, track_caller)]
    fn sub_assign(&mut self, rhs: usize) {
        self.offset -= rhs;
    }
}

impl<T> AddAssign<isize> for WithOffset<T> {
    #[cfg_attr(debug_assertions, track_caller)]
    fn add_assign(&mut self, rhs: isize) {
        self.offset = self.offset.wrapping_add_signed(rhs);
    }
}

impl<T> SubAssign<isize> for WithOffset<T> {
    #[cfg_attr(debug_assertions, track_caller)]
    fn sub_assign(&mut self, rhs: isize) {
        self.offset = self.offset.wrapping_add_signed(-rhs);
    }
}

impl<T> Add<usize> for WithOffset<T> {
    type Output = Self;

    #[cfg_attr(debug_assertions, track_caller)]
    fn add(mut self, rhs: usize) -> Self::Output {
        self += rhs;
        self
    }
}

impl<T> Sub<usize> for WithOffset<T> {
    type Output = Self;

    #[cfg_attr(debug_assertions, track_caller)]
    fn sub(mut self, rhs: usize) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<T> Add<isize> for WithOffset<T> {
    type Output = Self;

    #[cfg_attr(debug_assertions, track_caller)]
    fn add(mut self, rhs: isize) -> Self::Output {
        self += rhs;
        self
    }
}

impl<T> Sub<isize> for WithOffset<T> {
    type Output = Self;

    #[cfg_attr(debug_assertions, track_caller)]
    fn sub(mut self, rhs: isize) -> Self::Output {
        self -= rhs;
        self
    }
}
