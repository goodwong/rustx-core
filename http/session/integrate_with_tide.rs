/// 集成到tide
///
/// ```rs
/// use crate::http::session::integrate_with_tide::{CookieSession, RequestExt as RequestExtSession};
/// let session_middleware = CookieSession::new(&cipher_key);
/// let app = tide::new().middleware(session_middleware);
///
///
/// async fn handle_api(mut req: tide::Request<AppState>) -> tide::Result {
///     req.session.get::<String>("session_key").await?.ok_or("没有值")?;
///     req.session.set(key, value).await?;
///     // or
///     let session = req.session().clone();
/// }
/// ```
use super::{Session, SessionInner, SessionStatus};
use async_std::sync::RwLock;
use base64;
use chrono::Duration;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;
use tide::{http::Cookie, Next, Request};

type SessionHashMap = HashMap<String, String>;

const KEY_LENGTH: usize = 32;
const COOKIE_KEY: &str = "session";
const SESSION_LIFE_DAYS: i64 = 30;

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
                let value =
                    std::string::String::from_utf8(base64::decode(&raw_value).unwrap_or_default())
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
                (SessionStatus::Changed, Some(state)) | (SessionStatus::Renewed, Some(state)) => {
                    let state: SessionHashMap = state.collect();

                    // 编码
                    let raw_value = serde_json::to_string(&state)?;
                    // todo session cookie 还需要加密
                    // cipher...
                    // base64 编码，因为tide的cookie encode居然保留逗号，但是小程序不接受逗号
                    let value = base64::encode(raw_value.as_bytes());

                    debug!("session <= {} ({})", &value, &raw_value);
                    res.set_cookie(
                        Cookie::build(COOKIE_KEY, value)
                            .max_age(Duration::days(SESSION_LIFE_DAYS))
                            .http_only(true)
                            .finish(),
                    );
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
