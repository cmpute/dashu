use super::Inverse;

impl Inverse for f32 {
    type Output = f32;
    #[inline]
    fn inv(self) -> f32 {
        1.0 / self
    }
}

impl Inverse for &f32 {
    type Output = f32;
    #[inline]
    fn inv(self) -> f32 {
        1.0 / *self
    }
}

impl Inverse for f64 {
    type Output = f64;
    #[inline]
    fn inv(self) -> f64 {
        1.0 / self
    }
}

impl Inverse for &f64 {
    type Output = f64;
    #[inline]
    fn inv(self) -> f64 {
        1.0 / *self
    }
}
