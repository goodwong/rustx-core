#![allow(dead_code)]

use super::dingtalk::Dingtalk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

// 钉钉接口返回的类型定义
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    union_id: String,                  // "PiiiPyQqBNBii0HnCJ3zljcuAiEiE"，不会改变
    remark: String,                    // "remark",
    user_id: String,                   // "zhangsan"，创建后不可修改
    is_leader_in_depts: String,        // "{1:false}",
    is_boss: bool,                     // false,
    hired_date: u64,                   // 1520265600000,
    is_senior: bool,                   // false,
    tel: String,                       // "xxx-xxxxxxxx", 分机号（仅限企业内部开发调用）
    department: Vec<i32>,              // [1,2],
    work_place: String,                // "place",
    email: String,                     // "test@xxx.com",
    order_in_depts: String,            // "{1:71738366882504}",
    mobile: String,                    // "1xxxxxxxxxx", 手机号码
    active: bool,                      // false,
    avatar: String,                    // "xxx",
    is_admin: bool,                    // false, 是否为企业的管理员
    is_hide: bool,                     // false,
    job_number: String,                // "001",
    name: String,                      // "张三",
    ext_attr: HashMap<String, String>, // {}, 扩展属性，可以设置多种属性
    state_code: String,                // "86",
    position: String,                  // "manager",
    roles: Vec<UserRole>,              // [{"id": 149507744, "name": "总监", "groupName": "职务"}]
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserRole {
    id: u32,            //
    name: String,       // 角色名称
    group_name: String, // 角色组名称
}

impl Dingtalk {
    pub async fn user_info(&self, user_id: String) -> Result<UserInfo, Box<dyn Error>> {
        let url = "https://oapi.dingtalk.com/user/get?access_token=ACCESS_TOKEN&userid=USERID";
        self.post(url.replace("USERID", &user_id), &()).await
    }
}
