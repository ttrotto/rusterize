/* OS-dependent memory allocator for better performance */

#[cfg(not(target_family = "unix"))]
use mimalloc::MiMalloc;

#[cfg(not(target_family = "unix"))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(target_family = "unix")]
use tikv_jemallocator::Jemalloc;

#[cfg(target_family = "unix")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
