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
use tokio::sync::RwLock;

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
    token: RwLock<Option<Token>>,
    user: RwLock<Option<User>>,
    response: RwLock<Option<TokenResponse>>,
}
// 开放api
impl Identity {
    // 是否登录
    pub async fn is_login(&self) -> bool {
        self.get_token().await.is_some()
    }

    // 返回登录用户的id
    pub async fn user_id(&self) -> Option<i32> {
        self.get_token().await.map(|t| t.user_id as i32)
    }

    // 试图从数据库查询登陆的用户，并记住
    pub async fn user(&self) -> Option<User> {
        // 有await，这个写法不行
        //self.user.or_else(|| {
        //    self.token.map(async move |t| {
        //        self.user = find_user(t.user_id, self.config.db.get().ok()?).await.ok();
        //        self.user.clone()?
        //    })
        //})

        let exist = self.get_user().await;
        if exist.is_some() {
            return exist;
        }

        if let Some(token) = self.get_token().await {
            let uid = token.user_id as i32;
            let conn = self.config.db.get().ok()?;
            let user: Option<User> = find_user(uid, conn).await.ok();
            self.set_user(user.clone()).await;
            user
        } else {
            None
        }
    }

    // 登陆（在具体登陆的方式里调用该方法）
    pub async fn login(&self, user: User) -> AuthResult<()> {
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
        self.set_response(Some(TokenResponse::Set(token_str, expires)))
            .await;
        self.set_token(Some(token)).await;
        self.set_user(Some(user)).await;

        Ok(())
    }

    // 登出
    pub async fn logout(&self) -> AuthResult<()> {
        // 1. delete token in db
        if let Some(t) = self.get_token().await {
            destroy_refresh_token(t.refresh_token_id as i32, self.config.db.get()?).await?
        }

        // todo:
        // 2. mark the user logout, so the token will be outdated
        // 考虑
        //   1）https://docs.rs/ttl_cache
        //   2）如果cache容量不足需要淘汰数据时候，加入app的start_timestamp，
        //      要求token早于start_timestamp的都要求重新renew
        //      如果cache容量不足，可以将start_timestamp调后
        // ...

        // 3. set state
        self.set_response(Some(TokenResponse::Delete)).await;
        self.set_user(None).await;
        self.set_token(None).await;

        Ok(())
    }

    // 输出cookie
    pub async fn to_response(&self) -> Option<TokenResponse> {
        self.get_response().await
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
                    Ok(refresh_token) => {
                        if token.verify(&refresh_token) {
                            Self::with_renew(token, cfg).await
                        } else {
                            Ok(Self::with_invalid_token(cfg))
                        }
                    }
                }
            }
        }
    }
    fn with_none(config: Config) -> Self {
        Self {
            config,
            token: RwLock::new(None),
            user: RwLock::new(None),
            response: RwLock::new(None),
        }
    }
    fn with_invalid_token(config: Config) -> Self {
        Self {
            config,
            token: RwLock::new(None),
            user: RwLock::new(None),
            response: RwLock::new(Some(TokenResponse::Delete)),
        }
    }
    fn with_token(t: Token, config: Config) -> Self {
        Self {
            config,
            token: RwLock::new(Some(t)),
            user: RwLock::new(None),
            response: RwLock::new(None),
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
            token: RwLock::new(Some(token)),
            user: RwLock::new(None),
            response: RwLock::new(Some(TokenResponse::Set(token_str, expires))),
        })
    }

    async fn get_user(&self) -> Option<User> {
        self.user.read().await.clone()
    }
    async fn set_user(&self, user: Option<User>) {
        *self.user.write().await = user;
    }

    async fn get_token(&self) -> Option<Token> {
        *self.token.read().await
    }
    async fn set_token(&self, token: Option<Token>) {
        *self.token.write().await = token;
    }

    async fn get_response(&self) -> Option<TokenResponse> {
        self.response.read().await.clone()
    }
    async fn set_response(&self, response: Option<TokenResponse>) {
        *self.response.write().await = response;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenResponse {
    Set(String, DateTime<Utc>),
    Delete,
}

#[cfg(test)]
mod tests {
    use super::super::repository::{create_refresh_token, InsertToken};
    use super::super::token::{Token, TOKEN_LIFE_HOURS};
    use super::{AuthService, TokenResponse};
    use chrono::{Duration, Utc};

    use crate::auth::tests;
    use crate::auth::tests::TestResult;

    #[test]
    #[should_panic]
    fn invalid_cipher_key() {
        let pool = tests::db_pool();
        // should be panicked here
        // because of invalid key length
        AuthService::new(pool, "invalid key length");
    }

    #[tokio::test]
    async fn login() -> TestResult<()> {
        let pool = tests::db_pool();
        let auth = tests::auth_service(pool.clone());
        // 构建一个测试user
        let user = tests::mock_user(pool.clone()).await?;

        // todo 以下测试可以分拆到多个方法里面

        // 测试一：无效token
        let id = auth.get_identity("an invalid token").await?;
        assert_eq!(id.is_login().await, false);
        assert_eq!(id.user_id().await, None);
        assert_eq!(id.user().await, None);
        assert_eq!(id.to_response().await, Some(TokenResponse::Delete));

        // 测试三：登陆
        id.login(user.clone()).await?;
        assert_eq!(id.is_login().await, true);
        assert_eq!(user.id, id.user_id().await.unwrap());
        assert_eq!(user, id.user().await.unwrap());
        assert!(matches!(
            id.to_response().await,
            Some(TokenResponse::Set(_, _))
        ));
        println!("token: {:?}", id.to_response().await.unwrap());
        let _token_str = match id.to_response().await {
            Some(TokenResponse::Set(t, _)) => t,
            _ => "".to_string(),
        };

        // 测试四：登出
        id.logout().await?;
        assert_eq!(id.is_login().await, false);
        assert_eq!(id.user_id().await, None);
        assert_eq!(id.user().await, None);
        assert_eq!(id.to_response().await, Some(TokenResponse::Delete));

        // 测试五：使用登出的token（主动失效的token）
        // todo 登出后token应该失效
        /*
        let id = auth.get_identity(&_token_str).await?;
        assert_eq!(id.is_login().await, false);
        assert_eq!(id.user_id().await, None);
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
            .to_string(&auth.config.cipher_key)?;
            token_str
        };
        let id = auth.get_identity(&token_str).await?;
        assert_eq!(id.is_login().await, true);
        assert_eq!(user.id, id.user_id().await.unwrap());
        assert_eq!(user, id.user().await.unwrap());
        assert_eq!(id.to_response().await, None);

        // 测试六：过期的token（renew）
        let token_str = {
            // token
            let (nonce, hash) = Token::nonce_pair();
            let insert = InsertToken {
                user_id: user.id,
                device: Default::default(),
                hash,
            };
            let refresh_token = create_refresh_token(insert, pool.get()?).await?;

            // pack token string
            // !故意提前，让token过期，为了让数据库验证token
            let fake_issued_at = Utc::now() - Duration::hours(TOKEN_LIFE_HOURS + 1);
            let token = Token {
                nonce,
                user_id: user.id as i64,
                refresh_token_id: refresh_token.id as i64,
                issued_at: fake_issued_at.timestamp(),
            };
            let (token_str, _) = token.to_string(&auth.config.cipher_key)?;
            token_str
        };
        let id = auth.get_identity(&token_str).await?;
        assert_eq!(id.is_login().await, true);
        assert_eq!(user.id, id.user_id().await.unwrap());
        assert_eq!(user, id.user().await.unwrap());
        assert!(matches!(
            id.to_response().await,
            Some(TokenResponse::Set(_, _))
        ));
        if let Some(TokenResponse::Set(new_token_str, _)) = id.to_response().await {
            println!("renew token:");
            println!("old token: {}", token_str);
            println!("new token: {}", new_token_str);
        }

        // 测试七：再次使用renew前的token
        // 旧的token应该失效
        let id = auth.get_identity(&token_str).await?;
        assert_eq!(id.is_login().await, false);
        assert_eq!(id.user_id().await, None);
        assert_eq!(id.user().await, None);
        assert_eq!(id.to_response().await, Some(TokenResponse::Delete));

        Ok(())
    }
}
