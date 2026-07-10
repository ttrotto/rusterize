# This script copies the core crate + workspace manifest into src/ and rewrites the
# paths, making the built tarball self-contained for r-universe, CRAN,
# and devtools::install_github)

# 1. copy the core crate (../../rust) -> src/rusterize-rs
dir.create("src/rusterize-rs", showWarnings = FALSE)
core <- list.files("../../rust", full.names = TRUE, all.files = TRUE, no.. = TRUE)
core <- core[!basename(core) %in% c("target", "LICENSE")]
file.copy(core, "src/rusterize-rs", recursive = TRUE)

# 2. copy the workspace root manifest + lock -> src/
file.copy(c("../../Cargo.toml", "../../Cargo.lock"), "src/")

# 3. rewrite the copied workspace root
top <- "src/Cargo.toml"
l <- readLines(top)
l <- gsub('"rust"', '"rusterize-rs"', l, fixed = TRUE) # core crate
l <- gsub('"python", ', "", l, fixed = TRUE) # drop python member
l <- gsub('"R/rusterize/src/rust"', '"rust"', l, fixed = TRUE) # this R crate
writeLines(l, top)

# 4. point the R crate's path dependency at the copied-in core crate.
rc <- "src/rust/Cargo.toml"
l <- readLines(rc)
l <- gsub('path = "../../../../rust"', 'path = "../rusterize-rs"', l, fixed = TRUE)
writeLines(l, rc)

# 5. setup RUSTFLAGS
dir.create("src/.cargo", showWarnings = FALSE)
writeLines(
  c(
    '[target.\'cfg(target_arch = "x86_64")\']',
    'rustflags = ["-C", "target-feature=+sse3,+ssse3,+sse4.1,+sse4.2,+popcnt,+cmpxchg16b,+avx,+avx2,+fma,+bmi1,+bmi2,+lzcnt,+pclmulqdq,+movbe"]'
  ),
  "src/.cargo/config.toml"
)
