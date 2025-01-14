/*
OS-dependent memory allocator for better performance.
Adapted from https://github.com/pola-rs/polars/blob/main/py-polars/src/allocator.rs
 */

#[cfg(not(target_family = "unix"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(all(target_family = "unix", target_os = "macos"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
