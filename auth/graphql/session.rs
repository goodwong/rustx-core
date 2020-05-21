use anyhow::Error;
use async_std::sync::RwLock;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

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
impl Clone for Session {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

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

#[cfg(test)]
impl Default for Session {
    fn default() -> Self {
        Session(Arc::new(RwLock::new(SessionInner::default())))
    }
}

// 集成到 actix-session
// (将这部分与外部集成的代码独立出来，以后换掉的可能性有点大)
/*
mod integrate_with_actix_session {
    use super::{Session, SessionInner, SessionStatus};
    use actix_session::Session as ActixSession;
    use std::collections::HashMap;
    use std::sync::Arc;
    use async_std::sync::RwLock;

    type SessionHashMap = HashMap<String, String>;
    const SESSION_KEY: &str = "session";

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
}
*/

// 集成到tide
pub mod integrate_with_tide {
    use super::{Session, SessionInner, SessionStatus};
    use async_std::sync::RwLock;
    use base64;
    use futures::future::BoxFuture;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tide::{http::Cookie, Next, Request};

    type SessionHashMap = HashMap<String, String>;

    const KEY_LENGTH: usize = 32;
    const COOKIE_KEY: &str = "session";

    #[derive(Debug)]
    struct Config {
        cipher_key: [u8; KEY_LENGTH],
    }

    #[derive(Debug)]
    pub struct CookieSession {
        config: Config,
    }

    impl CookieSession {
        pub fn new(base64_encoded_key: &str) -> Self {
            use std::convert::TryInto;
            let cipher_key =
                base64::decode(base64_encoded_key).expect("SESSION_KEY must be base64 encoded");

            Self {
                config: Config {
                    cipher_key: cipher_key[..]
                        .try_into()
                        .unwrap_or_else(|_| panic!("session key LENGTH should be {}", KEY_LENGTH)),
                },
            }
        }
    }
    impl<State: Send + Sync + 'static> tide::Middleware<State> for CookieSession {
        fn handle<'a>(
            &'a self,
            req: Request<State>,
            next: Next<'a, State>,
        ) -> BoxFuture<'a, tide::Result> {
            Box::pin(async move {
                // parse from cookie
                let session = {
                    let raw_value = req
                        .cookie(COOKIE_KEY)
                        .map(|c: Cookie| c.value().to_owned())
                        .unwrap_or_default();

                    // 解析
                    let value = std::string::String::from_utf8(
                        base64::decode(&raw_value).unwrap_or_default(),
                    )
                    .unwrap_or_default();
                    debug!("session => {} ({})", &raw_value, &value);

                    // 转换为hashmap
                    let state: SessionHashMap = serde_json::from_str(&value).unwrap_or_default();

                    Session(Arc::new(RwLock::new(SessionInner {
                        state,
                        ..Default::default()
                    })))
                };

                // handler run
                let mut res = next.run(req.set_local(session.clone())).await?;

                // set to cookie response
                match session.get_changes().await {
                    (SessionStatus::Changed, Some(state))
                    | (SessionStatus::Renewed, Some(state)) => {
                        let state: SessionHashMap = state.collect();

                        // 编码
                        let raw_value = serde_json::to_string(&state)?;
                        // todo session cookie 还需要加密
                        // cipher...
                        // base64 编码，因为tide的cookie encode居然保留逗号，但是小程序不接受逗号
                        let value = base64::encode(raw_value.as_bytes());

                        debug!("session <= {} ({})", &value, &raw_value);
                        res.set_cookie(Cookie::new(COOKIE_KEY, value));
                    }
                    (SessionStatus::Purged, _) => {
                        res.remove_cookie(Cookie::named(COOKIE_KEY));
                        debug!("session <= removed!");
                    }
                    // todo: set a new session cookie upon first request (new client)
                    (SessionStatus::Unchanged, _) => (),
                    _ => (),
                }

                Ok(res)
            })
        }
    }
    pub trait RequestExt {
        fn session(&self) -> &Session;
    }
    impl<State> RequestExt for Request<State> {
        fn session(&self) -> &Session {
            self.local()
                .ok_or("CookieSession not initialized!")
                .unwrap()
        }
    }
}
