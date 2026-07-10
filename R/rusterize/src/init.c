
// clang-format sorts includes unless SortIncludes: Never. However, the ordering
// does matter here. So, we need to disable clang-format for safety.

// clang-format off
#include <stdint.h>
#include <Rinternals.h>
#include <R_ext/Parse.h>
// clang-format on

#include "rust/api.h"

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;

SEXP handle_result(SEXP res_) {
    uintptr_t res = (uintptr_t)res_;

    // An error is indicated by tag.
    if ((res & TAGGED_POINTER_MASK) == 1) {
        // Remove tag
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);

        // Currently, there are two types of error cases:
        //
        //   1. Error from Rust code
        //   2. Error from R's C API, which is caught by R_UnwindProtect()
        //
        if (TYPEOF(res_aligned) == CHARSXP) {
            // In case 1, the result is an error message that can be passed to
            // Rf_errorcall() directly.
            Rf_errorcall(R_NilValue, "%s", CHAR(res_aligned));
        } else {
            // In case 2, the result is the token to restart the
            // cleanup process on R's side.
            R_ContinueUnwind(res_aligned);
        }
    }

    return (SEXP)res;
}

SEXP savvy_rusterize_r__impl(SEXP c_arg__geometry, SEXP c_arg__raw_raster_info, SEXP c_arg__rpixel_fn, SEXP c_arg__rbackground, SEXP c_arg__all_touched, SEXP c_arg__encoding, SEXP c_arg__dtype, SEXP c_arg__rfield, SEXP c_arg__rby, SEXP c_arg__rburn) {
    SEXP res = savvy_rusterize_r__ffi(c_arg__geometry, c_arg__raw_raster_info, c_arg__rpixel_fn, c_arg__rbackground, c_arg__all_touched, c_arg__encoding, c_arg__dtype, c_arg__rfield, c_arg__rby, c_arg__rburn);
    return handle_result(res);
}

SEXP savvy_RArray_epsg__impl(SEXP self__) {
    SEXP res = savvy_RArray_epsg__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RArray_extent__impl(SEXP self__) {
    SEXP res = savvy_RArray_extent__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RArray_names__impl(SEXP self__) {
    SEXP res = savvy_RArray_names__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RArray_resolution__impl(SEXP self__) {
    SEXP res = savvy_RArray_resolution__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RArray_to_raster__impl(SEXP self__) {
    SEXP res = savvy_RArray_to_raster__ffi(self__);
    return handle_result(res);
}


static const R_CallMethodDef CallEntries[] = {
    {"savvy_rusterize_r__impl", (DL_FUNC) &savvy_rusterize_r__impl, 10},
    {"savvy_RArray_epsg__impl", (DL_FUNC) &savvy_RArray_epsg__impl, 1},
    {"savvy_RArray_extent__impl", (DL_FUNC) &savvy_RArray_extent__impl, 1},
    {"savvy_RArray_names__impl", (DL_FUNC) &savvy_RArray_names__impl, 1},
    {"savvy_RArray_resolution__impl", (DL_FUNC) &savvy_RArray_resolution__impl, 1},
    {"savvy_RArray_to_raster__impl", (DL_FUNC) &savvy_RArray_to_raster__impl, 1},
    {NULL, NULL, 0}
};

void R_init_rusterize(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);

    // Functions for initialization, if any.

}
