use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use turbo_sp1_program::program::TurboReducer;
use turbo_sp1_program::traits::TurboActionSerialization;

use crate::session::TurboSession;

pub struct SessionManager<PublicState, PrivateState, GameAction>
where
    PublicState: Send + Sync,
    PrivateState: Send + Sync,
    GameAction: Send + Sync,
{
    sessions: Arc<
        Mutex<HashMap<String, Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>>>,
    >,
}

impl<PublicState: Send + Sync, PrivateState: Send + Sync, GameAction: Send + Sync> Default
    for SessionManager<PublicState, PrivateState, GameAction>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<PublicState: Send + Sync, PrivateState: Send + Sync, GameAction: Send + Sync>
    SessionManager<PublicState, PrivateState, GameAction>
{
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<
        PublicState: Default + Send + Sync,
        PrivateState: Default + Send + Sync,
        GameAction: TurboActionSerialization + Send + Sync,
    > SessionManager<PublicState, PrivateState, GameAction>
{
    pub fn create_session(
        &self,
        reducer: TurboReducer<PublicState, PrivateState, GameAction>,
    ) -> Option<Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>> {
        let session = Arc::new(Mutex::new(TurboSession::new(reducer)));
        let id = session.lock().unwrap().id();
        self.sessions
            .lock()
            .unwrap()
            .insert(id.clone(), session.clone());
        Some(session)
    }

    pub fn get_session(
        &self,
        id: &str,
    ) -> Option<Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>> {
        self.sessions.lock().unwrap().get(id).cloned()
    }
}
