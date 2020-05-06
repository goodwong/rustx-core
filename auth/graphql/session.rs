use actix_session::Session as ActixSession;
use anyhow::Error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const SESSION_KEY: &str = "session";
/// The high-level interface you use to modify session data.
///
/// Session object could be obtained with
/// [`RequestSession::session`](trait.RequestSession.html#tymethod.session)
/// method. `RequestSession` trait is implemented for `HttpRequest`.
///
/// ```rust
/// use actix_session::Session;
/// use actix_web::*;
///
/// fn index(session: Session) -> Result<&'static str> {
///     // access session data
///     if let Some(count) = session.get::<i32>("counter")? {
///         session.set("counter", count + 1)?;
///     } else {
///         session.set("counter", 1)?;
///     }
///
///     Ok("Welcome!")
/// }
/// # fn main() {}
/// ```
pub struct Session(Arc<RwLock<SessionInner>>);

#[derive(PartialEq, Clone, Debug)]
pub enum SessionStatus {
    Changed,
    Purged,
    Renewed,
    Unchanged,
}
impl Default for SessionStatus {
    fn default() -> SessionStatus {
        SessionStatus::Unchanged
    }
}

#[derive(Default)]
struct SessionInner {
    state: HashMap<String, String>,
    pub status: SessionStatus,
}

impl Session {
    /// Get a `value` from the session.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        if let Some(s) = self.0.read().await.state.get(key) {
            Ok(Some(serde_json::from_str(s)?))
        } else {
            Ok(None)
        }
    }

    /// Set a `value` from the session.
    pub async fn set<T: Serialize>(&self, key: &str, value: T) -> Result<(), Error> {
        let mut inner = self.0.write().await;
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner
                .state
                .insert(key.to_owned(), serde_json::to_string(&value)?);
        }
        Ok(())
    }

    /// Remove value from the session.
    pub async fn remove(&self, key: &str) {
        let mut inner = self.0.write().await;
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.state.remove(key);
        }
    }

    /// Clear the session.
    pub async fn clear(&self) {
        let mut inner = self.0.write().await;
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Changed;
            inner.state.clear()
        }
    }

    /// Removes session, both client and server side.
    pub async fn purge(&self) {
        let mut inner = self.0.write().await;
        inner.status = SessionStatus::Purged;
        inner.state.clear();
    }

    /// Renews the session key, assigning existing session state to new key.
    pub async fn renew(&self) {
        let mut inner = self.0.write().await;
        if inner.status != SessionStatus::Purged {
            inner.status = SessionStatus::Renewed;
        }
    }

    /*
    pub async fn set_session(
        data: impl Iterator<Item = (String, String)>,
        req: &mut ServiceRequest,
    ) {
        let session = Session::get_session(&mut *req.extensions_mut());
        let mut inner = session.0.borrow_mut();
        inner.state.extend(data);
    }
    */

    pub async fn get_changes(
        &self,
    ) -> (
        SessionStatus,
        Option<impl Iterator<Item = (String, String)>>,
    ) {
        let mut session_mut = self.0.write().await;
        let state = std::mem::replace(&mut session_mut.state, HashMap::new());
        (session_mut.status.clone(), Some(state.into_iter()))
    }

    /*
    fn get_session(extensions: &mut Extensions) -> Session {
        if let Some(s_impl) = extensions.get::<Rc<RefCell<SessionInner>>>() {
            return Session(Rc::clone(&s_impl));
        }
        let inner = Rc::new(RefCell::new(SessionInner::default()));
        extensions.insert(inner.clone());
        Session(inner)
    }
    */
}

type SessionHashMap = HashMap<String, String>;

impl Session {
    pub fn from_request(req_session: &ActixSession) -> Self {
        let state = req_session
            .get::<SessionHashMap>(SESSION_KEY)
            .unwrap_or_else(|_| None)
            .unwrap_or_else(Default::default);
        debug!("session from_request(): {:?}", &state);

        Session(Arc::new(RwLock::new(SessionInner {
            state,
            ..Default::default()
        })))
    }

    pub async fn to_response(&self, req_session: &ActixSession) {
        match self.get_changes().await {
            (SessionStatus::Changed, Some(state)) | (SessionStatus::Renewed, Some(state)) => {
                let state: SessionHashMap = state.collect();
                debug!("session to_response(): {:?}", &state);

                req_session.set(SESSION_KEY, state).ok(); // ignore error result
            }
            (SessionStatus::Purged, _) => req_session.remove(SESSION_KEY),
            // todo: set a new session cookie upon first request (new client)
            (SessionStatus::Unchanged, _) => (),
            _ => (),
        }
    }
}

#[cfg(test)]
impl Default for Session {
    fn default() -> Self {
        Session(Arc::new(RwLock::new(SessionInner::default())))
    }
}
