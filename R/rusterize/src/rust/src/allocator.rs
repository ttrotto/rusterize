/* OS-dependent memory allocator for better performance */

// wasm supports neither jemalloc nor mimalloc; fall through to std default allocator
#[cfg(all(not(target_family = "unix"), not(target_arch = "wasm32")))]
use mimalloc::MiMalloc;

#[cfg(all(not(target_family = "unix"), not(target_arch = "wasm32")))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(target_family = "unix")]
use tikv_jemallocator::Jemalloc;

#[cfg(target_family = "unix")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
