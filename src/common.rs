use std::ops::Mul;

pub struct Size<T> {
    pub height: T,
    pub width: T,
}

impl<T> Mul<T> for Size<T>
where
    T: Mul<Output = T> + Copy,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Size {
            height: self.height * rhs,
            width: self.width * rhs,
        }
    }
}

pub type USize = Size<usize>;

impl<T> Size<T>
where
    T: Mul<Output = T> + Copy,
{
    pub fn square(&self) -> T {
        self.height * self.width
    }
}
