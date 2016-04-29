pub use std::f64::consts::PI;

pub type Time = f32;
/// A type alias for internal floating point precision.
pub type Float = f64;

/// Defines conversion methods from a plain `1/x` ratio into db and vice versa.
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
    assert_relative_eq!(0.5, Float::from_db(-6.0), epsilon = 0.015);
}
