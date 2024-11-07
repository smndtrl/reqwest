#[cfg(all(target_os = "wasi", any(target_env = "p1", target_env = "p2")))]
pub mod component;
#[cfg(all(target_os = "wasi", any(target_env = "p1", target_env = "p2")))]
pub use component::*;

#[cfg(not(all(target_os = "wasi", any(target_env = "p1", target_env = "p2"))))]
pub mod js;
#[cfg(not(all(target_os = "wasi", any(target_env = "p1", target_env = "p2"))))]
pub use js::*;
