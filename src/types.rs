use std::cell::RefCell;
use std::rc::Rc;

pub trait Wrap<T> {
    fn wrap(t: T) -> Self;
}

pub type SharedMut<T> = Rc<RefCell<T>>;
impl<T> Wrap<T> for SharedMut<T> {
    fn wrap(t: T) -> Self {
        Rc::new(RefCell::new(t))
    }
}

pub use std::f64::consts::PI;
pub use std::ops;

pub type Time = f32;
/// A type alias for internal floating point precision.
pub type Float = f64;

#[derive(Debug, Clone, Copy, PartialEq)]
/// A tuple struct that represents a single stereo frame.
pub struct Stereo(pub Float, pub Float);

/// Overload addition for `Stereo` frame.
impl ops::Add<Stereo> for Stereo {
    type Output = Stereo;

    fn add(self, rhs: Stereo) -> Self {
        Stereo(self.0 + rhs.0, self.1 + rhs.1)
    }
}
impl ops::Add<Float> for Stereo {
    type Output = Stereo;

    fn add(self, rhs: Float) -> Self {
        Stereo(self.0 + rhs, self.1 + rhs)
    }
}
impl ops::AddAssign<Stereo> for Stereo {
    fn add_assign(&mut self, rhs: Stereo) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}
/// Overload subtraction for `Stereo` frame.
impl ops::Sub<Stereo> for Stereo {
    type Output = Stereo;

    fn sub(self, rhs: Stereo) -> Self {
        Stereo(self.0 - rhs.0, self.1 - rhs.1)
    }
}
impl ops::Sub<Float> for Stereo {
    type Output = Stereo;

    fn sub(self, rhs: Float) -> Self {
        Stereo(self.0 - rhs, self.1 - rhs)
    }
}
impl ops::SubAssign<Stereo> for Stereo {
    fn sub_assign(&mut self, rhs: Stereo) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}
/// Overload multiplication for `Stereo` frame.
impl ops::Mul<Stereo> for Stereo {
    type Output = Stereo;

    fn mul(self, rhs: Stereo) -> Self {
        Stereo(self.0 * rhs.0, self.1 * rhs.1)
    }
}
impl ops::MulAssign<Stereo> for Stereo {
    fn mul_assign(&mut self, rhs: Stereo) {
        self.0 *= rhs.0;
        self.1 *= rhs.1;
    }
}
/// Overload multiplication for `Stereo` frame with scalar.
impl ops::Mul<Float> for Stereo {
    type Output = Stereo;

    fn mul(self, rhs: Float) -> Self {
        Stereo(self.0 * rhs, self.1 * rhs)
    }
}
impl ops::MulAssign<Float> for Stereo {
    fn mul_assign(&mut self, rhs: Float) {
        self.0 *= rhs;
        self.1 *= rhs;
    }
}
impl ops::Div<Float> for Stereo {
    type Output = Stereo;

    fn div(self, rhs: Float) -> Self {
        Stereo(self.0 / rhs, self.1 / rhs)
    }
}
impl ops::Div<Stereo> for Stereo {
    type Output = Stereo;

    fn div(self, rhs: Stereo) -> Self {
        Stereo(self.0 / rhs.0, self.1 / rhs.1)
    }
}
impl Default for Stereo {
    fn default() -> Self {
        Stereo(0.0, 0.0)
    }
}

#[test]
fn test_stereo() {
    // It would be better to make a relative equality check with some error epsilon. But
    // therefore `Sub` and  `abs()` had to be implemented for `Stereo`.
    let (a, b) = (Stereo(1.0, 2.0), Stereo(2.0, 4.0));
    assert_eq!(a + b, Stereo(3.0, 6.0));
    assert_eq!(a - b, Stereo(-1.0, -2.0));
    assert_eq!(b - a, Stereo(1.0, 2.0));
    assert_eq!(a * b, Stereo(2.0, 8.0));
    assert_eq!(a * 3.0, Stereo(3.0, 6.0));
    let mut x = Stereo::default();
    x += Stereo(5.0, 10.0);
    assert_eq!(x, Stereo(5.0, 10.0));
    x *= 0.1;
    assert_eq!(x, Stereo(0.5, 1.0));
}

pub const MINUS_THREE_DB: Float = 0.7079457843841379;

/// Defines conversion methods from a plain `1/x` power ratio into db and vice versa.
pub trait Db {
    /// Returns the ratio in dB.
    ///
    /// Example:
    /// `assert!(Db::to_rb(0.0001), -80.0)`
    fn to_db(ratio: Float) -> Float;
    /// Returns the `1/x` ratio from the given dB value.
    fn from_db(db: Float) -> Float;
}

impl Db for Float {
    fn to_db(ratio: Float) -> Float {
        20.0 * ratio.log10()
    }

    fn from_db(db: Float) -> Float {
        let ten: Float = 10.0;
        ten.powf(db / 20.0)
    }
}

#[test]
fn test_conversion() {
    assert_relative_eq!(-80.0, Float::to_db(0.0001));
    assert_relative_eq!(0.0, Float::to_db(1.0));
    assert_relative_eq!(6.0, Float::to_db(2.0), epsilon = 0.03);
    assert_relative_eq!(MINUS_THREE_DB * MINUS_THREE_DB, Float::from_db(-6.0));
    assert_relative_eq!(MINUS_THREE_DB, Float::from_db(-3.0));
}
