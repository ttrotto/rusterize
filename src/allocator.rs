/*
OS-dependent memory allocator for better performance.
Adapted from https://github.com/pola-rs/polars/blob/main/py-polars/src/allocator.rs
 */

#[cfg(all(
    any(not(target_family = "unix"), allocator = "mimalloc"),
    not(allocator = "default")
))]
use mimalloc::MiMalloc;

#[cfg(all(
    any(not(target_family = "unix"), allocator = "mimalloc"),
    not(allocator = "default")
))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(all(
    target_family = "unix",
    not(target_os = "macos"),
    not(allocator = "mimalloc"),
    not(allocator = "default")
))]
use jemallocator::Jemalloc;

#[cfg(all(
    target_family = "unix",
    not(target_os = "macos"),
    not(allocator = "mimalloc"),
    not(allocator = "default")
))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(all(
    target_family = "unix",
    target_os = "macos",
    not(allocator = "mimalloc"),
    not(allocator = "default")
))]
use jemallocator::Jemalloc;

#[cfg(all(
    target_family = "unix",
    target_os = "macos",
    not(allocator = "mimalloc"),
    not(allocator = "default")
))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
