auth 模块
==========

> 用于登陆认证的模块


**由以下部分组成：**
* graphql接口
    - 登陆（密码/短信/第三方……）
    - 登出
    - *renew续约
    - 查询当前登录态身份

* middleware拦截器
    - 登陆拦截
    - 自动续约renew


原理
------
**token**
1. token内容为加密的用户身份信息
2. 在短时内，直接解密并验证成功，无需查询数据库；
3. 超过了一段时间后，则在解密成功前提下，还需要去数据库验证，并发放新的token

**cookie**
1. cookie里带上token
1. 当token被刷新时候，自动通过cookie更新
3. 也可以通过api获取token，自动刷新token时候会占用一次接口请求，不推荐（暂不实现）

**“踢下线”**
1. 具有主动“踢”下线功能，马上生效，无需等待token的TTL过期
1. 支持多设备同时登陆（类似于微信的手机和电脑端）
1. 支持A处登陆，B处即刻下线（类似微信的电脑端和网页端）
1. 支持按设备“踢”下线（类似于坚果云）



新的思路（不同于之前go-x设计的那样）
-----------------------------
> 将token和refresh_token合而为一

原理：
1. 登陆，发放mix_token（里面包含类似jwt功能的token和refresh_token_id）
2. 请求时候header带上此 mix_token
3. 解密 --> 验证sign(AEAD可略过这步) --> 验证exp(或iat)
    1. 如果sign失效，返回401，重新登录
    2. 如果exp失效，查询数据库验证nonce是否有效

结构
-----
token：
   8位     8位      8位      12位      32位
          token  refresh  refresh    sha256
|--uid--|--exp--|--tid--|--nonce--|---sign--|
           或
           iat
*这里用iat为了检测提前失效token方便

供参考的：go-x（它的jwt token和refresh_token分开保存）
|-id(8)-|-nonce(12)-| (20字节)
NaCL加密+24 = 44
Base64 = 59字节

再改进：
> 其实token本身(不是数据库里的sign字段)不需要sign了，NaCL本身能保证数据不能篡改   
> 另外NaCL本身有一个nonce，所以nonce也不需要了  
> 这里的设计是token和refresh_token合并
新的结构如下：
token：
   8位     8位      8位  
|--uid--|--tid--|--iat--|
> tid即refresh_token_id
步骤：
1. 从cookie获取token后，解包，比较iat，如果在10分钟内，则验证通过
2. 解包失败，返回401
3. 如果iat过期，则通过tid+nonce验证查询数据库是否存在并有效
4. 如果不存在或无效，则返回401
5. 如果存在，验证nonce是否匹配sign，如果不匹配，返回401
5. 如果有效：
    1. 生成新的token
    2. 将新的nonce对应的sign更新至数据库（id/uid/device/expired_at不变，hash/issued_at变）
    3. 写入cookie并返回客户端



方案二：
```rs
// 初始化，一般在main.rs里
// Panics：1. 秘钥长度不对
let auth = ::authenticate::AuthService::new(db_pool, cipher_key); 

// 实例化
// 一般是在 http handler里处理
// 此identity可以放入graphql的context参数结构里去
// error: 一般是数据库连接问题，可以返回500
let identity = auth.from_request(token).await?; // req or cookie?


// 判断、获取数据
if identity.is_login() {}
if let Some(user_id) = identity.user_id() {}
if let Some(user) = identity.user().await {}
// 登陆（可能在graphql handler里面）
identity.login(user).await?; // 错误：1. 数据库错误
// 登出（可能在graphql handler里面）
identity.logout().await?; // 错误：1. 数据库错误


// to response
// 这个还是在 http handler里处理
// 一般是在graphql.execute后
match identity.to_response() {
    Some(TokenResponse::Set(value, expires)) => write_cookie(value, expires), // 登陆、更新
    Some(TokenResponse::Delete) => delete_cookie(), // 登出、失效
    None => _, // 未登录、已登录但未过期
}
```