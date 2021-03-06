use super::client::{Client, ClientResult, Config as ClientConfig};
use serde::{Deserialize, Serialize};

// dingtalk 配置
#[derive(Serialize, Deserialize, Debug, Default)]
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
    cfg: Config,
    client: Client,
}
impl Dingtalk {
    pub fn new(cfg: Config) -> Dingtalk {
        let token_url = "https://oapi.dingtalk.com/gettoken?appkey=KEY&appsecret=SECRET";
        let token_url = token_url
            .replace("KEY", &cfg.app_key)
            .replace("SECRET", &cfg.app_secret);
        let client = Client::new(ClientConfig { token_url });

        Dingtalk { cfg, client }
    }

    pub async fn access_token(&self) -> ClientResult<String> {
        self.client.access_token().await
    }

    #[cfg(test)]
    pub(crate) async fn set_invalid_access_token(&self) {
        self.client.set_invalid_access_token().await
    }
}

// 钉钉接口返回的类型定义
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    userid: String,             // "zhangsan"，创建后不可修改
    unionid: String,            // "PiiiPyQqBNBii0HnCJ3zljcuAiEiE"，不会改变
    name: String,               // "张三",
    tel: Option<String>,        // "xxx-xxxxxxxx", 分机号（仅限企业内部开发调用）
    work_place: Option<String>, // "place",
    remark: Option<String>,     // "remark",
    mobile: String,             // "1xxxxxxxxxx", 手机号码
    email: Option<String>,      // "test@xxx.com",
    org_email: Option<String>,  // "test@xxx.com",
    active: bool,               // false,
    order_in_depts: String,     // "{1:71738366882504}",
    is_admin: bool,             // false, 是否为企业的管理员
    is_boss: bool,              // false,
    is_leader_in_depts: String, // "{1:false}",
    is_hide: bool,              // false,
    department: Vec<i32>,       // [1,2],
    position: String,           // "manager",
    avatar: String,             // "xxx",
    hired_date: u64,            // 1520265600000,
    jobnumber: String,          // "001",
    // extattr: HashMap<String, String>, // {}, 扩展属性，可以设置多种属性
    is_senior: bool,      // false,
    state_code: String,   // "86",
    roles: Vec<UserRole>, // [{"id": 149507744, "name": "总监", "groupName": "职务"}]
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserRole {
    id: u32,            //
    name: String,       // 角色名称
    group_name: String, // 角色组名称
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Microapp {
    pub app_icon: String,                 // 应用图标
    pub agent_id: i32,                    // 应用实例化id
    pub app_desc: String,                 // 应用描述
    pub is_self: bool,                    // 表示是否是自建应用
    pub name: String,                     // 应用名称
    pub homepage_link: Option<String>,    // 应用的移动端主页
    pub pc_homepage_link: Option<String>, // 应用的pc端主页
    pub app_status: i32,                  // 应用状态， 1表示启用 0表示停用
    pub omp_link: Option<String>,         // 应用的OA后台管理主页
}
impl Dingtalk {
    pub async fn user_info(&self, user_id: String) -> ClientResult<UserInfo> {
        let url = "https://oapi.dingtalk.com/user/get?access_token=ACCESS_TOKEN&userid=USERID";
        self.client.get(&url.replace("USERID", &user_id)).await
    }
    pub async fn microapp_list(&self) -> ClientResult<Vec<Microapp>> {
        let url = "https://oapi.dingtalk.com/microapp/list?access_token=ACCESS_TOKEN";

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct ApiResult {
            app_list: Vec<Microapp>,
        }
        let resp = self.client.post::<(), ApiResult>(url, None).await?;
        Ok(resp.app_list)
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Dingtalk};
    type TestResult<O> = Result<O, Box<dyn std::error::Error + Send + Sync>>;

    fn setup() {
        // 为了在testing下看到logging
        env_logger::try_init().ok();
    }

    #[test]
    fn read_config_from_env() {
        setup();

        let cfg = Config::from_env();
        info!("dingtalk cfg: {:#?}", cfg);
    }

    #[async_std::test]
    async fn get_access_token() -> TestResult<()> {
        setup();

        let cfg = Config::from_env();
        let dd = Dingtalk::new(cfg);

        dbg!(dd.access_token().await?);
        Ok(())
    }

    #[async_std::test]
    #[ignore = "需要钉钉后台设置白名单ip才能测试"]
    async fn auto_refresh_access_token() -> TestResult<()> {
        setup();

        let dd = Dingtalk::new(Config::from_env());
        dd.access_token().await?;
        dd.set_invalid_access_token().await;
        let app_list = dd.microapp_list().await?;
        dbg!(app_list);
        Ok(())
    }

    #[async_std::test]
    #[ignore = "需要钉钉后台设置白名单ip才能测试"]
    async fn get_user_info() -> TestResult<()> {
        setup();

        let dd = Dingtalk::new(Config::from_env());
        let user_info = dd.user_info("manager7140".to_string()).await?;
        debug!("user_info: {:#?}", user_info);
        Ok(())
    }
}
