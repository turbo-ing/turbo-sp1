use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use turbo_sp1_program::program::TurboReducer;
use turbo_sp1_program::traits::TurboActionSerialization;
use uuid::Uuid;

use crate::session::TurboSession;

pub struct SessionManager<PublicState, PrivateState, GameAction>
where
    PublicState: Default + Send + Sync,
    PrivateState: Default + Send + Sync,
    GameAction: TurboActionSerialization + Send + Sync,
{
    sessions:
        Mutex<HashMap<String, Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>>>,
}

impl<
        PublicState: Default + Send + Sync,
        PrivateState: Default + Send + Sync,
        GameAction: TurboActionSerialization + Send + Sync,
    > Default for SessionManager<PublicState, PrivateState, GameAction>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<
        PublicState: Default + Send + Sync,
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
    ) -> Option<Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>> {
        let id = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.lock().await;
        sessions.insert(id.clone(), Arc::new(Mutex::new(TurboSession::new(reducer))));
        self.get_session(&id).await
    }

    pub async fn get_session(
        &self,
        id: &str,
    ) -> Option<Arc<Mutex<TurboSession<PublicState, PrivateState, GameAction>>>> {
        let sessions = self.sessions.lock().await;
        sessions.get(id).cloned()
    }
}
