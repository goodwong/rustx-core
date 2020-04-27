use super::{Dingtalk, DingtalkError};
use http::Method;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct AccessToken {
    token: String,
    expired_at: Instant,
}
impl AccessToken {
    pub(crate) fn new(token: String) -> AccessToken {
        AccessToken {
            token,
            expired_at: Instant::now() + Duration::from_secs(7200),
        }
    }
    pub(crate) fn default() -> AccessToken {
        AccessToken {
            token: "".to_string(),
            expired_at: Instant::now() - Duration::from_secs(10),
        }
    }
    pub(crate) fn valid(&self) -> bool {
        self.expired_at > Instant::now() + Duration::from_secs(10)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    access_token: String,
}
impl Dingtalk {
    async fn fetch_access_token(&self) -> Result<String, DingtalkError> {
        let url = "https://oapi.dingtalk.com/gettoken?appkey=KEY&appsecret=SECRET";
        let url = url
            .replace("KEY", &self.cfg.app_key)
            .replace("SECRET", &self.cfg.app_secret);
        let result: Response = Self::raw_request(Method::GET, url, &()).await?;
        Ok(result.access_token)
    }

    async fn get_token(&self) -> AccessToken {
        self.access_token.read().await.clone()
    }
    async fn set_token(&self, token: AccessToken) {
        *self.access_token.write().await = token
    }

    pub async fn access_token(&self) -> Result<String, DingtalkError> {
        let access_token = self.get_token().await;
        if !access_token.valid() {
            let new_token = self.fetch_access_token().await?;
            self.set_token(AccessToken::new(new_token)).await;
        }
        Ok(access_token.token.to_string())
    }

    pub async fn reset_access_token(&self, old_token: String) {
        let mut access_token = self.access_token.write().await;
        // 再次判断，避免排队reset
        // 如token已经变化，
        // 说明中间(因为锁，可能需要等待很久)有其他进程reset或fetch过这个token了，
        // 则不再reset
        if access_token.token == old_token {
            *access_token = AccessToken::default();
        }
    }
}
