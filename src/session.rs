use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone)]
pub struct Session {
    pub id: Uuid,
    pub data: HashMap<String, String>,
    pub last_activity: Instant,
}

pub struct SessionManager {
    pub sessions: HashMap<Uuid, Session>,
    pub timeout: Duration,
}

impl SessionManager {
    pub fn new(timeout_seconds: u64) -> Self {
        SessionManager {
            sessions: HashMap::new(),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    pub fn create_session(&mut self) -> Session {
        let id = Uuid::new_v4();
        let session = Session {
            id,
            data: HashMap::new(),
            last_activity: Instant::now(),
        };
        self.sessions.insert(id, session.clone());
        session
    }

    pub fn get_session(&mut self, id: &Uuid) -> Option<&mut Session> {
        if let Some(session) = self.sessions.get_mut(id) {
            session.last_activity = Instant::now();
            Some(session)
        } else {
            None
        }
    }

    pub fn clean_expired_sessions(&mut self) {
        let now = Instant::now();
        self.sessions
            .retain(|_, session| now.duration_since(session.last_activity) < self.timeout);
    }
}
