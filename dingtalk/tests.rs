use super::access_token::AccessToken;
use super::dingtalk::{Config, Dingtalk};
use tokio::runtime::Runtime;

#[test]
fn read_config_from_env() {
    let cfg = Config::from_env();
    println!("cfg: {:#?}", cfg);
}

#[test]
fn get_access_token() {
    let cfg = Config::from_env();
    let dd = Dingtalk::new(cfg);

    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        println!("access_token: {}", dd.access_token().await.unwrap());
    });
}

#[test]
fn get_user_info() {
    let dd = Dingtalk::new(Config::from_env());

    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let user_info = dd.user_info("manager7140".to_string()).await.unwrap();
        println!("user_info: {:#?}", user_info);
    });
}

#[test]
fn auto_refresh_access_token() {
    let dd = Dingtalk::new(Config::from_env());
    {
        let mut access_token = dd.access_token.lock().unwrap();
        *access_token = AccessToken::new("asdfasdfkajslkdfjals".to_string());
    }

    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let _ = dd.user_info("manager7140".to_string()).await.unwrap();
    });
}
