//#![allow(unused_imports)]
use super::error::AuthResult;
use super::models::User;
use super::repository::{
    create_refresh_token, destroy_refresh_token, find_refresh_token, find_user,
    renew_refresh_token, InsertToken,
};
use super::token::{Token, KEY_LENGTH};
use crate::db_connection::PgPool;

use base64;
use chrono::{DateTime, Utc};
use diesel::result::Error::NotFound;

pub struct AuthService {
    config: Config,
}
#[derive(Clone)]
pub struct Config {
    pub db: PgPool,
    pub cipher_key: [u8; KEY_LENGTH],
}
impl AuthService {
    /// 初始化，一般在main.rs里
    /// Panics：1. 秘钥长度不对
    /// ```rs
    /// let auth = ::authenticate::AuthService::new(db_pool, cipher_key);
    /// ```
    pub fn new(db: PgPool, base64_encoded_key: &str) -> Self {
        use std::convert::TryInto;
        let cipher_key =
            base64::decode(base64_encoded_key).expect("CIPHER_KEY must be base64 encoded");

        Self {
            config: Config {
                db,
                cipher_key: cipher_key[..]
                    .try_into()
                    .unwrap_or_else(|_| panic!("cipher key LENGTH should be {}", KEY_LENGTH)),
            },
        }
    }

    /// 实例化
    /// 一般是在 http handler里处理
    /// 此identity可以放入graphql的context参数结构里去
    /// error: 一般是数据库连接问题，可以返回500
    pub async fn get_identity(&self, token_str: &str) -> AuthResult<Identity> {
        Identity::from_request(self.config.clone(), token_str).await
    }
}

pub struct Identity {
    config: Config,
    token: Option<Token>,
    user: Option<User>,
    response: Option<TokenResponse>,
}
// 开放api
impl Identity {
    // 是否登录
    pub fn is_login(&self) -> bool {
        self.token.is_some()
    }

    // 返回登录用户的id
    pub fn user_id(&self) -> Option<i32> {
        self.token.as_ref().map(|t| t.user_id as i32)
    }

    // 试图从数据库查询登陆的用户，并记住
    pub async fn user(&mut self) -> Option<User> {
        // 有await，这个写法不行
        //self.user.or_else(|| {
        //    self.token.map(async move |t| {
        //        self.user = find_user(t.user_id, self.config.db.get().ok()?).await.ok();
        //        self.user.clone()?
        //    })
        //})

        match &self.user {
            Some(_) => self.user.clone(),
            None => match &self.token {
                Some(t) => {
                    let uid = t.user_id as i32;
                    let conn = self.config.db.get().ok()?;
                    self.user = find_user(uid, conn).await.ok();
                    self.user.clone()
                }
                _ => None,
            },
        }
    }

    // 登陆（在具体登陆的方式里调用该方法）
    pub async fn login(&mut self, user: User) -> AuthResult<()> {
        // token
        let (nonce, hash) = Token::nonce_pair();
        let insert = InsertToken {
            user_id: user.id,
            device: Default::default(),
            hash,
        };
        let refresh_token = create_refresh_token(insert, self.config.db.get()?).await?;

        // pack token string
        let token = Token {
            nonce,
            user_id: user.id as i64,
            refresh_token_id: refresh_token.id as i64,
            issued_at: refresh_token.issued_at.timestamp(),
        };
        // mut self
        let (token_str, expires) = token.to_string(&self.config.cipher_key)?;
        self.response = Some(TokenResponse::Set(token_str, expires));
        self.token = Some(token);
        self.user = Some(user);

        Ok(())
    }

    // 登出
    pub async fn logout(&mut self) -> AuthResult<()> {
        // 1. delete token in db
        let refresh_token_id = self
            .token
            .as_ref()
            .map(|t| t.refresh_token_id as i32)
            .ok_or("empty token")?;
        let conn = self.config.db.get()?;
        destroy_refresh_token(refresh_token_id, conn).await?;

        // todo:
        // 2. mark the user logout, so the token will be outdated
        // 考虑
        //   1）https://docs.rs/ttl_cache
        //   2）如果cache容量不足需要淘汰数据时候，加入app的start_timestamp，
        //      要求token早于start_timestamp的都要求重新renew
        //      如果cache容量不足，可以将start_timestamp调后
        // ...

        // 3. response delete cookie
        self.response = Some(TokenResponse::Delete);

        // 4. reset self
        self.user = None;
        self.token = None;

        Ok(())
    }
}
// 内部方法
impl Identity {
    /// 解析token string获取Identity。
    /// > 仅在数据库错误时候，返回Err
    async fn from_request(cfg: Config, token_str: &str) -> AuthResult<Self> {
        let key = cfg.cipher_key;
        match Token::from_string(token_str, &key) {
            // 解析失败
            Err(_) => Ok(Self::with_invalid_token(cfg)),

            // token有效
            Ok(token) if !token.is_expired() => Ok(Self::with_token(token, cfg)),

            // 过期的token，需要数据库验证
            Ok(token) => {
                let conn = cfg.db.get()?;
                match find_refresh_token(token.refresh_token_id as i32, conn).await {
                    Err(NotFound) => Ok(Self::with_invalid_token(cfg)),
                    Err(e) => Err(e.into()),
                    // 校验
                    Ok(refresh_token) => match refresh_token.is_valid(&token.nonce) {
                        true => Self::with_renew(token, cfg).await,
                        false => Ok(Self::with_invalid_token(cfg)),
                    },
                }
            }
        }
    }
    // 输出cookie
    pub fn to_response(&self) -> Option<TokenResponse> {
        self.response.clone()
    }
    fn with_none(config: Config) -> Self {
        Self {
            config,
            token: None,
            user: None,
            response: None,
        }
    }
    fn with_invalid_token(config: Config) -> Self {
        Self {
            config,
            token: None,
            user: None,
            response: Some(TokenResponse::Delete),
        }
    }
    fn with_token(t: Token, config: Config) -> Self {
        Self {
            config,
            token: Some(t),
            user: None,
            response: None,
        }
    }
    async fn with_renew(t: Token, config: Config) -> AuthResult<Self> {
        // 1. update refresh_token
        let (nonce, hash) = Token::nonce_pair();
        let iat = renew_refresh_token(t.refresh_token_id as i32, hash, config.db.get()?).await?;

        // 2. create new token
        let token = Token {
            nonce,
            issued_at: iat.timestamp(),
            ..t
        };

        // 3. set to response
        let (token_str, expires) = token.to_string(&config.cipher_key)?;
        Ok(Self {
            config,
            token: Some(token),
            user: None,
            response: Some(TokenResponse::Set(token_str, expires)),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenResponse {
    Set(String, DateTime<Utc>),
    Delete,
}

#[cfg(test)]
mod tests {
    use super::super::repository::{create_refresh_token, create_user, InsertToken, InsertUser};
    use super::super::token::{Token, TOKEN_LIFE_HOURS};
    use super::{AuthService, TokenResponse};
    use crate::db_connection::{establish_connection, PgPool};
    use chrono::{Duration, Utc};
    use dotenv::dotenv;
    use std::env;

    fn db_pool() -> PgPool {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        establish_connection(database_url)
    }

    #[test]
    #[should_panic]
    fn new_auth_service_invalid_cipher_key() {
        let pool = db_pool();
        // should be panicked here
        // because of invalid key length
        AuthService::new(pool, "invalid key length");
    }

    #[tokio::test]
    async fn test_login() {
        let pool = db_pool();
        let cipher_key = "12345678_2345678_2345678_2345678";
        let auth = AuthService::new(pool.clone(), cipher_key);
        // 构建一个测试user
        let user = {
            let conn = pool.get().unwrap();
            let _insert = InsertUser {
                username: "test".to_string() + &Utc::now().format("%s").to_string(),
                name: "test".to_string(),
                avatar: "test".to_string(),
            };
            create_user(_insert, conn).await.unwrap()
        };

        // 测试一：无效token
        let mut id = auth.get_identity("an invalid token").await.unwrap();
        assert_eq!(id.is_login(), false);
        assert_eq!(id.user_id(), None);
        assert_eq!(id.user().await, None);
        assert_eq!(id.to_response(), Some(TokenResponse::Delete));

        // 测试三：登陆
        id.login(user.clone()).await.unwrap();
        assert_eq!(id.is_login(), true);
        assert_eq!(user.id, id.user_id().unwrap());
        assert_eq!(user, id.user().await.unwrap());
        assert!(matches!(id.to_response(), Some(TokenResponse::Set(_, _))));
        println!("token: {:?}", id.to_response().unwrap());
        let _token_str = match id.to_response() {
            Some(TokenResponse::Set(t, _)) => t,
            _ => "".to_string(),
        };

        // 测试四：登出
        id.logout().await.unwrap();
        assert_eq!(id.is_login(), false);
        assert_eq!(id.user_id(), None);
        assert_eq!(id.user().await, None);
        assert_eq!(id.to_response(), Some(TokenResponse::Delete));

        // 测试五：使用登出的token（主动失效的token）
        // todo 登出后token应该失效
        /*
        let mut id = auth.get_identity(&_token_str).await.unwrap();
        assert_eq!(id.is_login(), false);
        assert_eq!(id.user_id(), None);
        assert_eq!(id.user().await, None);
        assert_eq!(id.to_response(), Some(TokenResponse::Delete));
        */

        // 测试二：有效token
        // 无需数据库验证
        let token_str = {
            let (nonce, _) = Token::nonce_pair();
            let (token_str, _) = Token {
                nonce,
                user_id: user.id as i64,
                refresh_token_id: Default::default(),
                issued_at: Utc::now().timestamp(),
            }
            .to_string(&auth.config.cipher_key)
            .unwrap();
            token_str
        };
        let mut id = auth.get_identity(&token_str).await.unwrap();
        assert_eq!(id.is_login(), true);
        assert_eq!(user.id, id.user_id().unwrap());
        assert_eq!(user, id.user().await.unwrap());
        assert_eq!(id.to_response(), None);

        // 测试六：过期的token（renew）
        let token_str = {
            // token
            let (nonce, hash) = Token::nonce_pair();
            let insert = InsertToken {
                user_id: user.id,
                device: Default::default(),
                hash,
            };
            let refresh_token = create_refresh_token(insert, pool.get().unwrap())
                .await
                .unwrap();

            // pack token string
            // !故意提前，让token过期，为了让数据库验证token
            let fake_issued_at = Utc::now() - Duration::hours(TOKEN_LIFE_HOURS + 1);
            let token = Token {
                nonce,
                user_id: user.id as i64,
                refresh_token_id: refresh_token.id as i64,
                issued_at: fake_issued_at.timestamp(),
            };
            let (token_str, _) = token.to_string(&auth.config.cipher_key).unwrap();
            token_str
        };
        let mut id = auth.get_identity(&token_str).await.unwrap();
        assert_eq!(id.is_login(), true);
        assert_eq!(user.id, id.user_id().unwrap());
        assert_eq!(user, id.user().await.unwrap());
        assert!(matches!(id.to_response(), Some(TokenResponse::Set(_, _))));
        if let Some(TokenResponse::Set(new_token_str, _)) = id.to_response() {
            println!("renew token:");
            println!("old token: {}", token_str);
            println!("new token: {}", new_token_str);
        }

        // 测试七：再次使用renew前的token
        // 旧的token应该失效
        let mut id = auth.get_identity(&token_str).await.unwrap();
        assert_eq!(id.is_login(), false);
        assert_eq!(id.user_id(), None);
        assert_eq!(id.user().await, None);
        assert_eq!(id.to_response(), Some(TokenResponse::Delete));
    }
}
