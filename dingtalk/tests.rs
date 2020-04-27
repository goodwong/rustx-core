use super::access_token::AccessToken;
use super::{Config, Dingtalk};

#[test]
fn read_config_from_env() {
    let cfg = Config::from_env();
    println!("cfg: {:#?}", cfg);
}

#[tokio::test]
async fn get_access_token() {
    let cfg = Config::from_env();
    let dd = Dingtalk::new(cfg);

    println!("access_token: {}", dd.access_token().await.unwrap());
}

#[tokio::test]
async fn get_user_info() {
    let dd = Dingtalk::new(Config::from_env());

    let user_info = dd.user_info("manager7140".to_string()).await.unwrap();
    println!("user_info: {:#?}", user_info);
}

#[tokio::test]
async fn auto_refresh_access_token() {
    let dd = Dingtalk::new(Config::from_env());
    {
        let mut access_token = dd.access_token.write().await;
        *access_token = AccessToken::new("asdfasdfkajslkdfjals".to_string());
    }

    let _ = dd.user_info("manager7140".to_string()).await.unwrap();
}
