use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use turbo_program::program::TurboReducer;
use turbo_program::traits::TurboActionSerialization;

use crate::session::TurboSession;

pub struct SessionManager<PublicState, PrivateState, GameAction>
where
    PublicState: Serialize + Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    sessions:
        Mutex<HashMap<String, Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>>>,
}

impl<
        PublicState: Serialize + Default + Send + Sync,
        PrivateState: Default + Send + Sync,
        GameAction: TurboActionSerialization + Send + Sync,
    > Default for SessionManager<PublicState, PrivateState, GameAction>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<
        PublicState: Serialize + Default + Send + Sync,
        PrivateState: Default + Send + Sync,
        GameAction: TurboActionSerialization + Send + Sync,
    > SessionManager<PublicState, PrivateState, GameAction>
{
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn create_session(
        &mut self,
        reducer: TurboReducer<PublicState, PrivateState, GameAction>,
    ) -> String {
        let session = TurboSession::new(reducer);
        let id = session.id();

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session.id(), Arc::new(Mutex::new(session)));
        id
    }

    pub async fn get_session(
        &self,
        id: &str,
    ) -> Option<Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>> {
        let sessions = self.sessions.lock().await;
        sessions.get(id).cloned()
    }
}
