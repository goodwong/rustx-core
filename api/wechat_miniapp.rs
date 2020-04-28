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
        let token_url = "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid=APPID&secret=APPSECRET";
        let token_url = token_url
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
    pub async fn msg_sec_check(&self, content: &str) -> ClientResult<()> {
        let url = "https://api.weixin.qq.com/wxa/msg_sec_check?access_token=ACCESS_TOKEN";
        let mut payload = HashMap::new();
        payload.insert("content", content);

        #[derive(Serialize, Deserialize, Debug)]
        struct ApiErrorResponse {
            errcode: Option<i32>,
            errmsg: Option<String>,
        }
        self.client
            .post::<_, ApiErrorResponse>(url, &payload)
            .await?;
        Ok(())
    }
    pub async fn code_to_session(&self, code: &str) -> ClientResult<Code2SessionResponse> {
        let url = "https://api.weixin.qq.com/sns/jscode2session?appid=APPID&secret=SECRET&js_code=JSCODE&grant_type=authorization_code";
        let url = url
            .replace("APPID", &self.cfg.appid)
            .replace("SECRET", &self.cfg.secret)
            .replace("JSCODE", code);
        self.client.get::<Code2SessionResponse>(&url).await
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Code2SessionResponse {
    openid: String,          //	用户唯一标识
    session_key: String,     //	会话密钥
    unionid: Option<String>, //	用户在开放平台的唯一标识符，在满足 UnionID 下发条件的情况下会返回，详见 UnionID 机制说明。
}

#[cfg(test)]
mod tests {
    use super::{Config, Miniapp};
    type TestResult<O> = Result<O, Box<dyn std::error::Error + Send + Sync>>;

    #[test]
    fn read_config_from_env() {
        let cfg = Config::from_env();
        println!("cfg: {:#?}", cfg);
    }

    #[tokio::test]
    async fn get_access_token() -> TestResult<()> {
        let cfg = Config::from_env();
        let app = Miniapp::new(cfg);

        println!("access_token: {}", app.access_token().await?);
        Ok(())
    }

    #[tokio::test]
    async fn auto_refresh_access_token() -> TestResult<()> {
        let app = Miniapp::new(Config::from_env());
        app.access_token().await?;
        app.set_invalid_access_token().await;

        let _ = app.msg_sec_check("平安顺利").await?;
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn msg_sec_check() {
        let app = Miniapp::new(Config::from_env());

        let _ = app.msg_sec_check("法轮功").await.unwrap();
    }

    #[tokio::test]
    async fn code_to_session() -> TestResult<()> {
        // 为了在testing下看到logging
        use env_logger;
        env_logger::init();

        use std::env;
        let code = env::var("JS_CODE").expect("value `JS_CODE` not set");
        let app = Miniapp::new(Config::from_env());
        let session = app.code_to_session(&code).await?;
        println!("session: {:?}", session);

        Ok(())
    }
}
