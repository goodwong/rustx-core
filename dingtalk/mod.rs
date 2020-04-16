#![allow(dead_code)]

mod access_token;
mod user;

#[cfg(test)]
mod tests;

use access_token::AccessToken;
use http::Method;
use reqwest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Mutex; // todo 改用tokio::sync::RwLock
use thiserror::Error as ThisError;

// dingtalk 配置
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub corp_id: String,
    pub agent_id: u64,
    pub app_key: String,
    pub app_secret: String,
}
impl Config {
    pub fn from_env() -> Config {
        use dotenv::dotenv;
        use std::env;

        dotenv().ok();
        let corp_id = env::var("DINGTALK_CORP_ID")
            .expect("value `DINGTALK_CORP_ID` not presented in .env file");
        let agent_id = env::var("DINGTALK_AGENT_ID")
            .expect("value `DINGTALK_AGENT_ID` not presented in .env file")
            .parse()
            .expect("value `DINGTALK_AGENT_ID` in .env file is not a valid integer");
        let app_key = env::var("DINGTALK_APP_KEY")
            .expect("value `DINGTALK_APP_KEY` not presented in .env file");
        let app_secret = env::var("DINGTALK_APP_SECRET")
            .expect("value `DINGTALK_APP_SECRET` not presented in .env file");

        Config {
            corp_id,
            agent_id,
            app_key,
            app_secret,
        }
    }
}

// dingtalk 结构
pub struct Dingtalk {
    pub(crate) cfg: Config,
    pub(crate) access_token: Mutex<AccessToken>,
}
impl Dingtalk {
    pub fn new(cfg: Config) -> Dingtalk {
        Dingtalk {
            cfg,
            access_token: Mutex::new(AccessToken::default()),
        }
    }

    pub(super) async fn raw_request<T, O>(
        &self,
        method: Method,
        url: String,
        payload: &T,
    ) -> Result<O, DingtalkError>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        // request...
        let client = reqwest::Client::new();
        let builder = match method {
            Method::POST => client.post(&url),
            Method::GET => client.get(&url),
            _ => panic!(
                "Dingtalk::raw_request do not support this Mehtod type: {}",
                method
            ),
        };
        let response = builder.json(payload).send().await?.text().await?;
        debug!("raw_request() response: {}", response);

        // check error...
        let error: ErrorResponse = serde_json::from_str(&response)?;
        match error.errcode {
            0 => (),
            -1 => return Err(DingtalkError::SystemBusy),
            40001 | 40014 | 41001 => return Err(DingtalkError::InvalidAccessToken),
            _ => {
                return Err(DingtalkError::Other(
                    "Api 返回：".to_string() + &error.errmsg,
                ))
            }
        }

        // deserialize...
        let result: O = serde_json::from_str(&response)?;
        Ok(result)
    }

    // 自动处理access_token
    async fn request<T, O>(
        &self,
        method: Method,
        url: String,
        payload: &T,
    ) -> Result<O, DingtalkError>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        // auto retry...
        let mut retry = 0;
        loop {
            // get access_token
            let access_token = if url.contains("ACCESS_TOKEN") {
                self.access_token().await?
            } else {
                "".to_string()
            };
            let url = url.clone().replace("ACCESS_TOKEN", &access_token);
            let result = self.raw_request(method.clone(), url.clone(), payload).await;

            retry += 1;
            if retry > 2 {
                break result;
            }

            // 除了无效token和系统繁忙需要进入重试，其它情况无论成功与否都退出循环
            match &result {
                Err(DingtalkError::InvalidAccessToken) => {
                    self.reset_access_token(access_token);
                }
                Err(DingtalkError::SystemBusy) => {
                    continue;
                }
                Ok(_) => break result,
                Err(_) => break result,
            }
        }
    }

    pub async fn get<O>(&self, url: String) -> Result<O, DingtalkError>
    where
        O: DeserializeOwned,
    {
        self.request(Method::GET, url, &()).await
    }

    pub async fn post<T, O>(&self, url: String, payload: &T) -> Result<O, DingtalkError>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        self.request(Method::POST, url, payload).await
    }
}

// api错误结构
#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    errcode: i32,
    errmsg: String,
}

// 错误类型
#[derive(ThisError, Debug)]
pub enum DingtalkError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("Dingtalk Error InvalidAccessToken")]
    InvalidAccessToken,
    #[error("Dingtalk Error SystemBusy")]
    SystemBusy,
    #[error("Dingtalk Error:")]
    Other(String),
}
