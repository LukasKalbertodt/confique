use super::*;

fn de<'de, T: serde::Deserialize<'de>>(v: &'static str) -> Result<T, DeError> {
    T::deserialize(Deserializer { value: v.into() })
}


#[test]
fn boolean() {
    assert_eq!(de("1"), Ok(true));
    assert_eq!(de("true "), Ok(true));
    assert_eq!(de(" True "), Ok(true));
    assert_eq!(de("  TRUE"), Ok(true));
    assert_eq!(de("yes"), Ok(true));
    assert_eq!(de(" Yes"), Ok(true));
    assert_eq!(de("YES "), Ok(true));

    assert_eq!(de("0  "), Ok(false));
    assert_eq!(de(" false"), Ok(false));
    assert_eq!(de(" False "), Ok(false));
    assert_eq!(de("FALSE "), Ok(false));
    assert_eq!(de("no"), Ok(false));
    assert_eq!(de(" No"), Ok(false));
    assert_eq!(de("NO "), Ok(false));
}

#[test]
fn ints() {
    assert_eq!(de("0"), Ok(0u8));
    assert_eq!(de("-1 "), Ok(-1i8));
    assert_eq!(de(" 27"), Ok(27u16));
    assert_eq!(de("-27"), Ok(-27i16));
    assert_eq!(de("   4301"), Ok(4301u32));
    assert_eq!(de(" -123456"), Ok(-123456i32));
    assert_eq!(de(" 986543210    "), Ok(986543210u64));
    assert_eq!(de("-986543210"), Ok(-986543210i64));
}

#[test]
fn floats() {
    assert_eq!(de("3.1415"), Ok(3.1415f32));
    assert_eq!(de("-123.456"), Ok(-123.456f64));
}
