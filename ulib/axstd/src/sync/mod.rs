//! Useful synchronization primitives.

#[doc(no_inline)]
pub use core::sync::atomic;

#[cfg(feature = "alloc")]
#[doc(no_inline)]
pub use alloc::sync::{Arc, Weak};

#[cfg(feature = "multitask")]
mod mutex;
#[cfg(feature = "multitask")]
mod condvar;

#[cfg(feature = "multitask")]
#[doc(cfg(feature = "multitask"))]
pub use self::mutex::{Mutex, MutexGuard};
#[cfg(feature = "multitask")]
#[doc(cfg(feature = "multitask"))]
pub use self::condvar::Condvar;

#[cfg(not(feature = "multitask"))]
#[doc(cfg(not(feature = "multitask")))]
pub use kspin::{SpinRaw as Mutex, SpinRawGuard as MutexGuard}; // never used in IRQ context
