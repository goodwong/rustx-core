use super::client::{Client, ClientResult, Config as ClientConfig};
//use anyhow::Result as AnyhowResult;
use super::aes_cbc_128;
use base64;
use serde::{Deserialize, Serialize};
use std::str;

type AnyhowResult<O> = Result<O, Box<dyn std::error::Error + Send + Sync>>;

// miniprogram 配置
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

// miniprogram 结构
pub struct Miniprogram {
    cfg: Config,
    client: Client,
}
impl Miniprogram {
    pub fn new(cfg: Config) -> Miniprogram {
        let token_url = "https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid=APPID&secret=APPSECRET";
        let token_url = token_url
            .replace("APPID", &cfg.appid)
            .replace("APPSECRET", &cfg.secret);
        let client = Client::new(ClientConfig { token_url });

        Miniprogram { cfg, client }
    }

    pub async fn access_token(&self) -> ClientResult<String> {
        self.client.access_token().await
    }

    #[cfg(test)]
    pub(crate) async fn set_invalid_access_token(&self) {
        self.client.set_invalid_access_token().await
    }
}

impl Miniprogram {
    pub async fn msg_sec_check(&self, content: &str) -> ClientResult<()> {
        let url = "https://api.weixin.qq.com/wxa/msg_sec_check?access_token=ACCESS_TOKEN";

        #[derive(Serialize)]
        struct MsgSecCheckRequest<'a> {
            content: &'a str,
        }
        #[derive(Deserialize)]
        struct ApiErrorResponse {
            errcode: Option<i32>,
            errmsg: Option<String>,
        }

        let payload = MsgSecCheckRequest { content };
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

    pub fn get_phone_number(
        session_key: &str,
        iv: &str,
        data: &str,
    ) -> AnyhowResult<PhoneNumberResult> {
        let session_key = base64::decode(session_key)?;
        let iv = base64::decode(iv)?;
        let data = base64::decode(data)?;

        let decrypted_data = aes_cbc_128::decrypt(&data, &session_key, &iv)
            .map_err(|e| format!("解密失败：{:?}", e))?;

        let data = str::from_utf8(&decrypted_data)?;
        serde_json::from_str::<PhoneNumberResult>(data).map_err(Into::into)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Code2SessionResponse {
    openid: String,          //	用户唯一标识
    session_key: String,     //	会话密钥
    unionid: Option<String>, //	用户在开放平台的唯一标识符，在满足 UnionID 下发条件的情况下会返回，详见 UnionID 机制说明。
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PhoneNumberResult {
    phone_number: String,      // 用户绑定的手机号（国外手机号会有区号）
    pure_phone_number: String, // 没有区号的手机号
    country_code: String,      //
}

#[cfg(test)]
mod tests {
    use super::{Config, Miniprogram};
    use std::env;
    type TestResult<O> = Result<O, Box<dyn std::error::Error + Send + Sync>>;

    #[test]
    fn read_config_from_env() {
        let cfg = Config::from_env();
        println!("cfg: {:#?}", cfg);
    }

    #[tokio::test]
    async fn get_access_token() -> TestResult<()> {
        let cfg = Config::from_env();
        let app = Miniprogram::new(cfg);

        println!("access_token: {}", app.access_token().await?);
        Ok(())
    }

    #[tokio::test]
    async fn auto_refresh_access_token() -> TestResult<()> {
        let app = Miniprogram::new(Config::from_env());
        app.access_token().await?;
        app.set_invalid_access_token().await;

        let _ = app.msg_sec_check("平安顺利").await?;
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn msg_sec_check() {
        let app = Miniprogram::new(Config::from_env());

        let _ = app.msg_sec_check("法轮功").await.unwrap();
    }

    #[tokio::test]
    async fn code_to_session() -> TestResult<()> {
        // 为了在testing下看到logging
        use env_logger;
        env_logger::init();

        let code = env::var("JS_CODE").expect("code_to_session `JS_CODE` not set");
        let app = Miniprogram::new(Config::from_env());
        let session = app.code_to_session(&code).await?;
        println!("session: {:?}", session);

        Ok(())
    }

    #[test]
    fn get_phone_number() {
        let session_key = "V76yDG9WkjT/ZRBOHaaw/Q==";
        let iv = "C5JfTNchCZl+Np3FzpNGZg==";
        let data = "QfvgdpP7cs7G/6uW135ygEw+C1FP5BQcoKnl8O+bSBwoeo0iNV62jF/5Y2+zrLrUxjppgx2+s+GlM8F6WURuNYGpD1uZpygOKMeSY6bo41QOlkyAa+H8DtNGp2fMnBgal/kP0ILvgfqnDuc5zUE3kjV1HFQNkQgMhIA4HsGm4r+d3C4sSebiAEvMxWs/f07ivPeaeBKPkFf/+PMpcNl0/A==";
        let result = Miniprogram::get_phone_number(session_key, iv, data).unwrap();
        println!("get_phone_number(): {:?}", result);
    }
}
