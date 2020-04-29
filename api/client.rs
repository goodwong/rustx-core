use chrono::{DateTime, Duration, Utc};
use http::Method;
use reqwest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error as ThisError;
use tokio::sync::RwLock;

// 配置
// 1. access_token 的response 2. varify错误类型
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub token_url: String,
}
impl Config {}

// Client 结构
pub struct Client {
    cfg: Config,
    token: AccessToken,
}
impl Client {
    pub fn new(cfg: Config) -> Client {
        Client {
            cfg,
            token: Default::default(),
        }
    }

    pub async fn get<O>(&self, url: &str) -> ClientResult<O>
    where
        O: DeserializeOwned,
    {
        self.request(Method::GET, url, &()).await
    }

    pub async fn post<T, O>(&self, url: &str, payload: &T) -> ClientResult<O>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        self.request(Method::POST, url, payload).await
    }

    pub async fn access_token(&self) -> ClientResult<String> {
        let token = self.token.get_token().await;
        if !token.valid() {
            let new_token = self.fetch_access_token().await?;
            self.token.set_token(new_token.clone()).await;
            Ok(new_token.access_token)
        } else {
            Ok(token.access_token)
        }
    }

    pub async fn reset_access_token(&self, old_token: String) {
        let mut token = self.token.0.write().await;
        // 再次判断，避免排队reset
        // 如token已经变化，
        // 说明中间(因为锁，可能需要等待很久)有其他进程reset或fetch过这个token了，
        // 则不再reset
        if token.access_token == old_token {
            *token = AccessTokenEntiry::default();
        }
    }

    #[cfg(test)]
    pub(crate) async fn set_invalid_access_token(&self) {
        let mut token = self.token.0.write().await;
        token.access_token = "invalid_access_token".to_owned();
    }
}

impl Client {
    async fn raw_request<T, O>(method: Method, url: &str, payload: &T) -> ClientResult<O>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        // request...
        let client = reqwest::Client::new();
        let builder = match method {
            Method::POST => client.post(url),
            Method::GET => client.get(url),
            _ => panic!("Invalid Mehtod: {}", method),
        };
        debug!("\t=> raw_request() send: {} {}", method, url);
        let response = builder.json(payload).send().await?.text().await?;
        debug!("\t<= raw_request() response: {}", response);

        // check error...
        Self::check_error(&response)?;

        // deserialize...
        serde_json::from_str(&response).map_err(|e| {
            warn!("serde_json解析错误：{:?}，response: {}", e, &response);
            e.into()
        })
    }

    fn check_error(response: &str) -> ClientResult<()> {
        // api错误结构
        #[derive(Serialize, Deserialize, Debug)]
        struct ApiErrorResponse {
            errcode: Option<i32>,
            errmsg: Option<String>,
        }

        let error: ApiErrorResponse = serde_json::from_str(response)?;
        match error.errcode {
            None => Ok(()),
            Some(0) => Ok(()),
            Some(-1) => Err(ClientError::SystemBusy),
            Some(40001) | Some(40014) | Some(41001) => Err(ClientError::InvalidToken),
            Some(_) => Err(ClientError::Other(format!("{:?}", error))),
        }?;
        Ok(())
    }

    // 自动处理access_token
    async fn request<T, O>(&self, method: Method, url: &str, payload: &T) -> ClientResult<O>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        // auto retry...
        let mut retry = 0;
        loop {
            // get access_token string
            let token_str = if url.contains("ACCESS_TOKEN") {
                self.access_token().await?
            } else {
                Default::default()
            };
            let result = {
                let url = url.replace("ACCESS_TOKEN", &token_str);
                Self::raw_request(method.clone(), &url, payload).await
            };

            retry += 1;
            if retry > 2 {
                break result;
            }

            // 除了无效token和系统繁忙需要进入重试，其它情况无论成功与否都退出循环
            match &result {
                Err(ClientError::InvalidToken) => warn!("InvalidToken! retry({}) <-{}", retry, url),
                Err(ClientError::SystemBusy) => warn!("SystemBusy! retry({}) <-{}", retry, url),
                Err(err) => warn!("ApiError: {} <-{}", err, url),
                Ok(_) => (),
            }
            match &result {
                Err(ClientError::InvalidToken) => self.reset_access_token(token_str).await,
                Err(ClientError::SystemBusy) => continue,
                Err(_) => break result,
                Ok(_) => break result,
            }
        }
    }

    async fn fetch_access_token(&self) -> ClientResult<AccessTokenEntiry> {
        #[derive(Serialize, Deserialize, Debug)]
        struct ApiTokenResponse {
            access_token: String,
            expires_in: Option<i64>, // 钉钉没有这个字段
        }

        let result: ApiTokenResponse =
            Self::raw_request(Method::GET, &self.cfg.token_url, &()).await?;
        let expires_in = Duration::seconds(result.expires_in.unwrap_or(7200)); // todo 钉钉固定7200，其它平台须注意此处
        info!("fetch_access_token() -> {}", &result.access_token);

        let token = AccessTokenEntiry::new(result.access_token, expires_in);
        Ok(token)
    }
}

// 错误类型
#[derive(ThisError, Debug)]
pub enum ClientError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::error::Error),
    #[error("Client Error InvalidToken")]
    InvalidToken,
    #[error("Client Error SystemBusy")]
    SystemBusy,
    #[error("Client Error:")]
    Other(String),
}
pub type ClientResult<T> = Result<T, ClientError>;

struct AccessToken(RwLock<AccessTokenEntiry>);
impl Default for AccessToken {
    fn default() -> Self {
        AccessToken(RwLock::new(AccessTokenEntiry::default()))
    }
}
impl AccessToken {
    async fn get_token(&self) -> AccessTokenEntiry {
        self.0.read().await.clone()
    }
    async fn set_token(&self, token: AccessTokenEntiry) {
        *self.0.write().await = token
    }
}

#[derive(Debug, Clone)]
struct AccessTokenEntiry {
    access_token: String,
    expired_at: DateTime<Utc>,
}
impl Default for AccessTokenEntiry {
    fn default() -> AccessTokenEntiry {
        AccessTokenEntiry {
            access_token: "".to_string(),
            expired_at: Utc::now() - Duration::seconds(10),
        }
    }
}
impl AccessTokenEntiry {
    pub(crate) fn new(access_token: String, ttl: Duration) -> AccessTokenEntiry {
        AccessTokenEntiry {
            access_token,
            expired_at: Utc::now() + ttl,
        }
    }
    pub(crate) fn valid(&self) -> bool {
        self.expired_at > Utc::now() + Duration::seconds(10)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn new_api_request() {
        //let api_request = Client::new(Config{
        //
        //})
    }
}
