use savvy::{Error as SavvyError, NumericScalar, NumericSexp, Result as SavvyResult, Sexp, StringSexp};

/// A generic structure wrapping a type extracted from a generic [`savvy::Sexp`].
pub(crate) struct FromR<T>(pub(crate) T);

impl<T> TryFrom<Sexp> for FromR<Option<T>>
where
    T: TryFrom<Sexp, Error = SavvyError>,
{
    type Error = SavvyError;

    fn try_from(value: Sexp) -> SavvyResult<FromR<Option<T>>> {
        if value.is_null() {
            Ok(FromR(None))
        } else {
            T::try_from(value).map(|v| FromR(Some(v)))
        }
    }
}

impl TryFrom<NumericSexp> for FromR<Vec<i32>> {
    type Error = SavvyError;

    fn try_from(value: NumericSexp) -> SavvyResult<FromR<Vec<i32>>> {
        Ok(FromR(value.as_slice_i32()?.to_vec()))
    }
}

impl TryFrom<NumericSexp> for FromR<Vec<f64>> {
    type Error = SavvyError;

    fn try_from(value: NumericSexp) -> SavvyResult<FromR<Vec<f64>>> {
        Ok(FromR(value.as_slice_f64().to_vec()))
    }
}

impl<T> TryFrom<NumericScalar> for FromR<Option<T>>
where
    FromR<T>: TryFrom<NumericScalar, Error = SavvyError>,
{
    type Error = SavvyError;

    fn try_from(value: NumericScalar) -> SavvyResult<FromR<Option<T>>> {
        FromR::<T>::try_from(value).map(|w| FromR(Some(w.0)))
    }
}

macro_rules! impl_from_numeric_scalar {
    ($($t:ty),* $(,)?) => {$(
        impl TryFrom<Sexp> for FromR<$t> {
            type Error = SavvyError;

            fn try_from(value: Sexp) -> SavvyResult<FromR<$t>> {
                NumericScalar::try_from(value).and_then(FromR::<$t>::try_from)
            }
        }
    )*};
}

impl_from_numeric_scalar!(usize, i32, f64);

impl TryFrom<Sexp> for FromR<Vec<String>> {
    type Error = SavvyError;

    fn try_from(value: Sexp) -> SavvyResult<FromR<Vec<String>>> {
        StringSexp::try_from(value).and_then(FromR::<Vec<String>>::try_from)
    }
}

impl TryFrom<StringSexp> for FromR<Vec<String>> {
    type Error = SavvyError;

    fn try_from(value: StringSexp) -> SavvyResult<FromR<Vec<String>>> {
        let strings = value.to_vec().into_iter().map(|s| s.to_string()).collect();
        Ok(FromR(strings))
    }
}

impl TryFrom<NumericScalar> for FromR<usize> {
    type Error = SavvyError;

    fn try_from(value: NumericScalar) -> SavvyResult<FromR<usize>> {
        let n = value.as_usize()?;
        Ok(FromR(n))
    }
}

impl TryFrom<NumericScalar> for FromR<i32> {
    type Error = SavvyError;

    fn try_from(value: NumericScalar) -> SavvyResult<FromR<i32>> {
        let n = value.as_i32()?;
        Ok(FromR(n))
    }
}

impl TryFrom<NumericScalar> for FromR<f64> {
    type Error = SavvyError;

    fn try_from(value: NumericScalar) -> SavvyResult<FromR<f64>> {
        Ok(FromR(value.as_f64()))
    }
}
