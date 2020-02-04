#![allow(dead_code)]

use super::access_token::AccessToken;
use reqwest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::error::Error;
use std::sync::Mutex;

// dingtalk 实例
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
        use std::env;
        let corp_id = env::var("DINGTALK_CORP_ID").unwrap();
        let agent_id = env::var("DINGTALK_AGENT_ID").unwrap();
        let app_key = env::var("DINGTALK_APP_KEY").unwrap();
        let app_secret = env::var("DINGTALK_APP_SECRET").unwrap();

        Config {
            corp_id,
            agent_id: agent_id.parse().unwrap(),
            app_key,
            app_secret,
        }
    }
}

pub struct Dingtalk {
    pub(crate) cfg: Config,
    pub(crate) access_token: Mutex<AccessToken>,
}
pub(super) enum RequestType {
    Post,
    Get,
}
#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    errcode: Option<i32>,
    errmsg: Option<String>,
}
impl Dingtalk {
    pub fn new(cfg: Config) -> Dingtalk {
        Dingtalk {
            cfg: cfg,
            access_token: Mutex::new(AccessToken::default()),
        }
    }

    pub(super) async fn raw_request<T, O>(
        &self,
        request_type: RequestType,
        url: String,
        payload: &T,
    ) -> Result<O, Box<dyn Error>>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        let client = reqwest::Client::new();
        let builder = match request_type {
            RequestType::Post => client.post(&url),
            RequestType::Get => client.get(&url),
        };
        let resp_text = builder.json(payload).send().await?.text().await?;

        // check error
        let error: ErrorResponse = serde_json::from_str(&resp_text)?;
        if let Some(errcode) = error.errcode {
            if errcode != 0 {
                return Err(From::from(error.errmsg.unwrap()));
            }
        }
        // deserialize...
        let result: O = serde_json::from_str(&resp_text)?;
        Ok(result)
    }

    // 自动处理access_token
    async fn request<T, O>(
        &self,
        request_type: RequestType,
        url: String,
        payload: &T,
    ) -> Result<O, Box<dyn Error>>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        let mut url = url;
        if url.contains("ACCESS_TOKEN") {
            let access_token = self.access_token().await?;
            url = url.replace("ACCESS_TOKEN", &access_token);
        }
        self.raw_request(request_type, url, payload).await
    }

    pub async fn get<O>(&self, url: String) -> Result<O, Box<dyn Error>>
    where
        O: DeserializeOwned,
    {
        self.request(RequestType::Get, url, &()).await
    }

    pub async fn post<T, O>(&self, url: String, payload: &T) -> Result<O, Box<dyn Error>>
    where
        T: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        self.request(RequestType::Post, url, payload).await
    }
}
