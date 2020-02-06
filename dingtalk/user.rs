#![allow(dead_code)]

use super::dingtalk::{Dingtalk, DingtalkError};
use serde::{Deserialize, Serialize};
// use std::collections::HashMap;

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

impl Dingtalk {
    pub async fn user_info(&self, user_id: String) -> Result<UserInfo, DingtalkError> {
        let url = "https://oapi.dingtalk.com/user/get?access_token=ACCESS_TOKEN&userid=USERID";
        self.get(url.replace("USERID", &user_id)).await
    }
}
