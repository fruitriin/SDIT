use std::collections::HashMap;

use super::session::{Session, SessionId};

/// 全セッションの管理とID採番を行う。
///
/// Window との紐付けは sdit バイナリ側（`SditApp`）が行う。
/// sdit-core は GUI に依存しないため、winit の `WindowId` を知らない。
pub struct SessionManager {
    sessions: HashMap<SessionId, Session>,
    next_id: u64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { sessions: HashMap::new(), next_id: 0 }
    }

    /// 次のセッション ID を採番する。
    pub fn next_id(&mut self) -> SessionId {
        let id = SessionId(self.next_id);
        self.next_id = self.next_id.checked_add(1).expect("SessionId overflow");
        id
    }

    /// セッションを登録する。
    pub fn insert(&mut self, session: Session) {
        self.sessions.insert(session.id, session);
    }

    /// セッションを取得する。
    pub fn get(&self, id: SessionId) -> Option<&Session> {
        self.sessions.get(&id)
    }

    /// セッションを削除する。
    pub fn remove(&mut self, id: SessionId) -> Option<Session> {
        self.sessions.remove(&id)
    }

    /// 登録されているセッション数を返す。
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// セッションが空かどうかを返す。
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_id_increments() {
        let mut mgr = SessionManager::new();
        assert_eq!(mgr.next_id(), SessionId(0));
        assert_eq!(mgr.next_id(), SessionId(1));
        assert_eq!(mgr.next_id(), SessionId(2));
    }

    #[test]
    fn test_empty_by_default() {
        let mgr = SessionManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
    }
}
