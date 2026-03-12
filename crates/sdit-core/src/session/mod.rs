pub mod persistence;
#[allow(clippy::module_inception)]
pub mod session;
pub mod sidebar;
pub mod window_registry;

pub use persistence::{AppSnapshot, SessionSnapshot, WindowGeometry};
pub use session::{Session, SessionId, SpawnParams, TerminalState};
pub use sidebar::SidebarState;
pub use window_registry::SessionManager;
