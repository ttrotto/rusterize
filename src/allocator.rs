/* OS-dependent memory allocator for better performance */

#[cfg(not(target_family = "unix"))]
use mimalloc::MiMalloc;

#[cfg(not(target_family = "unix"))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(all(target_family = "unix", target_os = "macos"))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(target_family = "unix", target_os = "macos"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
