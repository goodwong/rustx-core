#![allow(dead_code)]

use super::dingtalk::{Dingtalk, RequestType};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::{Duration, Instant};

#[derive(Debug)]
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
struct AccessTokenResponse {
    access_token: String,
}
impl Dingtalk {
    async fn fetch_access_token(&self) -> Result<String, Box<dyn Error>> {
        let url = "https://oapi.dingtalk.com/gettoken?appkey=KEY&appsecret=SECRET";
        let url = url
            .replace("KEY", &self.cfg.app_key)
            .replace("SECRET", &self.cfg.app_secret)
            .to_string();
        let result: AccessTokenResponse = self.raw_request(RequestType::Get, url, &()).await?;
        Ok(result.access_token)
    }

    pub async fn access_token(&self) -> Result<String, Box<dyn Error>> {
        let mut access_token = self.access_token.lock().unwrap();
        if !access_token.valid() {
            let new_token = self.fetch_access_token().await?;
            *access_token = AccessToken::new(new_token);
        }
        return Ok(access_token.token.to_string());
    }
}
