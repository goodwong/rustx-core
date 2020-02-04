use super::dingtalk::{Config, Dingtalk};

#[test]
fn read_config_from_env() {
    let cfg = Config::from_env();
    println!("cfg: {:#?}", cfg);
}

#[test]
fn get_access_token() {
    use tokio::runtime::Runtime;
    let cfg = Config::from_env();
    let dd = Dingtalk::new(cfg);

    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        println!("access_token: {}", dd.access_token().await.unwrap());
    });
}
