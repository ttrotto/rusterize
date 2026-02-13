/* Prepare dataframe for rasterization */

use crate::prelude::PolarsHandler;
use num_traits::Num;
use polars::prelude::*;

#[allow(clippy::too_many_arguments)]
pub fn cast_df<T>(
    df: Option<DataFrame>,
    field_name: Option<&str>,
    by_name: Option<&str>,
    burn_value: T,
    burn_length: usize,
) -> DataFrame
where
    T: Num + PolarsHandler,
{
    match df {
        None => {
            // case 1: create a dummy `field`
            DataFrame::new(vec![burn_value.into_column("field_casted", burn_length)]).unwrap()
        }
        Some(df) => {
            let mut lf = df.lazy();
            match (field_name, by_name) {
                (Some(field_col), Some(by_col)) => {
                    // case 2: both `field` and `by` specified
                    let (new_field_col, new_by_col) = if field_col != by_col {
                        lf = lf.rename([field_col, by_col], ["field_casted", "by_str"], true);
                        ("field_casted", "by_str")
                    } else {
                        lf = lf.rename([field_col], ["field_casted"], true);
                        ("field_casted", "field_casted")
                    };

                    lf = lf.with_columns([
                        col(new_field_col).cast(T::polars_dtype()).alias("field_casted"),
                        col(new_by_col).cast(DataType::String).alias("by_str"),
                    ]);
                }
                (Some(field_col), None) => {
                    // case 3: only `field` specified
                    lf = lf.rename([field_col], ["field_casted"], true);
                    lf = lf.with_column(col("field_casted").cast(T::polars_dtype()).alias("field_casted"));
                }
                (None, Some(by_col)) => {
                    // case 4: only `by` specified
                    lf = lf.rename([by_col], ["by_str"], true);
                    lf = lf.with_columns([
                        lit(burn_value).alias("field_casted"), // dummy `field`
                        col("by_str").cast(DataType::String).alias("by_str"),
                    ]);
                }
                (None, None) => {
                    // this should not happen
                    panic!("No columns specified for the given dataframe.")
                }
            }

            // collect casted dataframe
            lf.collect().unwrap()
        }
    }
}
