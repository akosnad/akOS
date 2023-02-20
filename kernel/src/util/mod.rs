//! Utitilies, structures used by the kernel

pub mod spinlock;
pub mod static_list;

pub use spinlock::Spinlock;
pub use static_list::StaticList;
