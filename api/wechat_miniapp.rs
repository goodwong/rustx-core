use super::client::{Client, ClientResult, Config as ClientConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// miniapp 配置
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub appid: String,
    pub secret: String,
}
impl Config {
    pub fn from_env() -> Config {
        use dotenv::dotenv;
        use std::env;

        dotenv().ok();
        let appid = env::var("WECHAT_WEAPP_APPID")
            .expect("value `WECHAT_WEAPP_APPID` not presented in .env file");
        let secret = env::var("WECHAT_WEAPP_SECRET")
            .expect("value `WECHAT_WEAPP_SECRET` not presented in .env file");

        Config { appid, secret }
    }
}

// miniapp 结构
pub struct Miniapp {
    cfg: Config,
    client: Client,
}
impl Miniapp {
    pub fn new(cfg: Config) -> Miniapp {
        let url = "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid=APPID&secret=APPSECRET";
        let token_url = url
            .replace("APPID", &cfg.appid)
            .replace("APPSECRET", &cfg.secret);
        let client = Client::new(ClientConfig { token_url });

        Miniapp { cfg, client }
    }

    pub async fn access_token(&self) -> ClientResult<String> {
        self.client.access_token().await
    }

    #[cfg(test)]
    pub(crate) async fn set_invalid_access_token(&self) {
        self.client.set_invalid_access_token().await
    }
}

impl Miniapp {
    pub async fn msg_sec_check(&self, content: String) -> ClientResult<()> {
        let url = "https://api.weixin.qq.com/wxa/msg_sec_check?access_token=ACCESS_TOKEN";
        let mut payload = HashMap::new();
        payload.insert("content", content);

        #[derive(Serialize, Deserialize, Debug)]
        struct ApiErrorResponse {
            errcode: Option<i32>,
            errmsg: Option<String>,
        }
        let _: ApiErrorResponse = self.client.post(url, &payload).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Miniapp};

    #[test]
    fn read_config_from_env() {
        let cfg = Config::from_env();
        println!("cfg: {:#?}", cfg);
    }

    #[tokio::test]
    async fn get_access_token() {
        let cfg = Config::from_env();
        let app = Miniapp::new(cfg);

        println!("access_token: {}", app.access_token().await.unwrap());
    }

    #[tokio::test]
    async fn auto_refresh_access_token() {
        let app = Miniapp::new(Config::from_env());
        app.access_token().await.unwrap();
        app.set_invalid_access_token().await;

        let _ = app.msg_sec_check("平安顺利".to_string()).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn msg_sec_check() {
        let app = Miniapp::new(Config::from_env());

        let _ = app.msg_sec_check("法轮功".to_string()).await.unwrap();
    }
}
