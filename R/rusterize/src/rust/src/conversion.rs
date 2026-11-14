use savvy::{
    Error as SavvyError, NotAvailableValue, NumericScalar, NumericSexp, Result as SavvyResult, Sexp, StringSexp,
    savvy_err,
};

/// A generic structure wrapping a type extracted from a generic [`savvy::Sexp`].
pub(crate) struct FromR<T>(pub(crate) T);

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
        value
            .iter()
            .map(|v| {
                if v.is_na() {
                    Err(savvy_err!("Found NA in `by` column. Consider dropping them."))
                } else {
                    Ok(v.to_string())
                }
            })
            .collect::<SavvyResult<Vec<String>>>()
            .map(FromR)
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
