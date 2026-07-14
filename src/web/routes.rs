use axum::{
    extract::{Query, State, Form},
    response::{Html, Redirect, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::{CookieJar, Cookie};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tera::{Tera, Context};

use crate::service::{UserService, RelationService, MomentService};
use crate::domain::Role;

// ============================================================
// 共享状态
// ============================================================
pub struct AppState {
    pub tera: Tera,
    pub user_service: Arc<UserService>,
    pub relation_service: Arc<RelationService>,
    pub moment_service: Arc<MomentService>,
}

// ============================================================
// 表单结构体（HTML 表单序列化）
// ============================================================
#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub uid: u64,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct RegisterForm {
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct MomentForm {
    pub content: String,
}

#[derive(serde::Deserialize)]
pub struct MomentEditForm {
    pub moment_id: u64,
    pub content: String,
}

#[derive(serde::Deserialize)]
pub struct MomentDeleteForm {
    pub moment_id: u64,
}

#[derive(serde::Deserialize)]
pub struct CommentForm {
    pub moment_id: u64,
    pub content: String,
}

#[derive(serde::Deserialize)]
pub struct CommentDeleteForm {
    pub comment_id: u64,
}

#[derive(serde::Deserialize)]
pub struct ProfileForm {
    pub name: String,
    pub gender: String,
    pub birth_date: String,
}

#[derive(serde::Deserialize)]
pub struct SearchForm {
    pub query: String,
}

#[derive(serde::Deserialize)]
pub struct FriendActionForm {
    pub target_id: u64,
}

#[derive(serde::Deserialize)]
pub struct GroupForm {
    pub group_name: String,
}

#[derive(serde::Deserialize)]
pub struct MoveFriendForm {
    pub friend_id: u64,
    pub group_name: String,
}

#[derive(serde::Deserialize)]
pub struct AdminCancelForm {
    pub uid: u64,
}

#[derive(serde::Deserialize)]
pub struct AdminMomentDeleteForm {
    pub moment_id: u64,
}

// ============================================================
// 工具函数
// ============================================================

/// 从 Cookie 中提取登录会话
fn get_session(jar: &CookieJar) -> Option<(u64, Role)> {
    let uid = jar.get("user_id")?.value().parse::<u64>().ok()?;
    let role_byte = jar.get("role")?.value().parse::<u8>().ok()?;
    let role = match role_byte {
        0 => Role::User,
        1 => Role::Admin,
        _ => return None,
    };
    Some((uid, role))
}

/// 构建模板上下文的基础字段（用户态 + 消息）
fn base_context(user_id: Option<u64>, role: Option<Role>, msg: &str, msg_type: &str) -> Context {
    let mut ctx = Context::new();
    if let Some(uid) = user_id {
        ctx.insert("user_id", &uid);
    }
    if let Some(r) = role {
        ctx.insert("role", &(r as u8));
    }
    if !msg.is_empty() {
        ctx.insert("msg", msg);
        ctx.insert("msg_type", msg_type);
    }
    ctx
}

/// 从 Query 参数中提取 flash message
fn flash_from_query(params: &HashMap<String, String>) -> (String, String) {
    let msg = params.get("msg").cloned().unwrap_or_default();
    let msg_type = params.get("type").cloned().unwrap_or_default();
    (msg, msg_type)
}

/// 渲染 Tera 模板，包裹为 HTML 响应
fn render(tera: &Tera, template: &str, ctx: &Context) -> Response {
    match tera.render(template, ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            // 打印完整错误链到 stderr
            eprintln!("=== 模板渲染错误 ===");
            eprintln!("模板: {}", template);
            eprintln!("错误: {}", e);
            eprintln!("详细: {:?}", e);
            if let Some(source) = e.source() {
                eprintln!("原因: {}", source);
            }
            // 也显示到页面上
            Html(format!(
                "<h1>模板错误</h1>\
                 <p>模板: <code>{}</code></p>\
                 <pre>{}</pre>\
                 <hr>\
                 <details><summary>Debug 详情</summary><pre>{:?}</pre></details>",
                template, e, e
            )).into_response()
        }
    }
}

// ============================================================
// 认证相关路由
// ============================================================

/// GET / —— 登录/注册页（已登录则跳到首页）
async fn login_page(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if get_session(&jar).is_some() {
        return Redirect::to("/home").into_response();
    }
    let (msg, msg_type) = flash_from_query(&params);
    let mut ctx = base_context(None, None, &msg, &msg_type);
    ctx.insert("title", "登录 / 注册");
    render(&state.tera, "login.html", &ctx)
}

/// POST /login
async fn login_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    match state.user_service.verify_login(form.uid, &form.password).await {
        Ok(role) => {
            // 登录成功：写入 Cookie（30 天有效期）
            let jar = jar.add(Cookie::build(("user_id", form.uid.to_string()))
                .path("/")
                .max_age(time::Duration::days(30)));
            let jar = jar.add(Cookie::build(("role", (role as u8).to_string()))
                .path("/")
                .max_age(time::Duration::days(30)));
            // 重定向到角色对应页面
            let target = match role {
                Role::Admin => "/admin",
                Role::User => "/home",
            };
            (jar, Redirect::to(target)).into_response()
        }
        Err(e) => {
            let mut ctx = base_context(None, None, &e.to_string(), "error");
            ctx.insert("title", "登录 / 注册");
            render(&state.tera, "login.html", &ctx)
        }
    }
}

/// POST /register
async fn register_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RegisterForm>,
) -> Response {
    match state.user_service.create_user(&form.password).await {
        Ok(new_id) => {
            let msg = format!("注册成功！您的用户 ID 是 {}，请牢记。", new_id);
            Redirect::to(&format!("/?msg={}&type=success", urlencoding(&msg))).into_response()
        }
        Err(e) => {
            let mut ctx = base_context(None, None, &e.to_string(), "error");
            ctx.insert("title", "登录 / 注册");
            render(&state.tera, "login.html", &ctx)
        }
    }
}

/// GET /logout
async fn logout_handler(jar: CookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from("user_id"));
    let jar = jar.remove(Cookie::from("role"));
    (jar, Redirect::to("/?msg=已退出登录&type=success"))
}

/// 简单的 URL 编码帮助函数
fn urlencoding(s: &str) -> String {
    s.replace(' ', "+")
        .replace('\n', "%0A")
}

// ============================================================
// 首页 / 朋友圈路由
// ============================================================

/// GET /home —— 朋友圈动态流
async fn home_page(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };

    let (msg, msg_type) = flash_from_query(&params);

    // 获取好友动态，默认取前 20 条
    let moments = state.moment_service.get_friends_moments(uid, 20, 0).await
        .unwrap_or_else(|e| {
            eprintln!("获取动态失败: {}", e);
            vec![]
        });

    let profile = state.user_service.get_profile(uid).await.ok();

    let mut ctx = base_context(Some(uid), Some(role), &msg, &msg_type);
    ctx.insert("title", "朋友圈");
    ctx.insert("moments", &moments);
    if let Some(ref p) = profile {
        ctx.insert("user_name", &p.name);
    }
    render(&state.tera, "home.html", &ctx)
}

/// POST /moments —— 发布新动态
async fn post_moment_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<MomentForm>,
) -> Response {
    let (uid, _) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };

    match state.moment_service.post_moment(uid, &form.content).await {
        Ok(_) => Redirect::to("/home?msg=发布成功！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/home?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /moments/edit —— 编辑自己的动态
async fn edit_moment_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<MomentEditForm>,
) -> Response {
    let (uid, _) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };

    match state.moment_service.update_moment(uid, form.moment_id, &form.content).await {
        Ok(_) => Redirect::to("/home?msg=编辑成功！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/home?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /moments/delete —— 删除自己的动态
async fn delete_moment_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<MomentDeleteForm>,
) -> Response {
    let (uid, _) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };

    match state.moment_service.delete_moment(uid, form.moment_id).await {
        Ok(_) => Redirect::to("/home?msg=删除成功！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/home?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /comments —— 发表评论
async fn post_comment_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<CommentForm>,
) -> Response {
    let (uid, _) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };

    match state.moment_service.post_comment(uid, form.moment_id, &form.content).await {
        Ok(_) => Redirect::to("/home?msg=评论成功！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/home?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /comments/delete —— 删除自己的评论
async fn delete_comment_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<CommentDeleteForm>,
) -> Response {
    let (uid, _) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };

    match state.moment_service.delete_comment(uid, form.comment_id).await {
        Ok(_) => Redirect::to("/home?msg=评论已删除！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/home?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

// ============================================================
// 个人资料路由
// ============================================================

/// GET /profile —— 查看和编辑个人资料
async fn profile_page(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需设置个人资料&type=error").into_response();
    }

    let (msg, msg_type) = flash_from_query(&params);

    let profile = match state.user_service.get_profile(uid).await {
        Ok(p) => p,
        Err(e) => {
            return Redirect::to(&format!("/?msg={}&type=error", urlencoding(&e.to_string()))).into_response();
        }
    };

    let mut ctx = base_context(Some(uid), Some(role), &msg, &msg_type);
    ctx.insert("title", "个人资料");
    ctx.insert("profile", &profile);
    render(&state.tera, "profile.html", &ctx)
}

/// POST /profile —— 保存个人资料
async fn profile_save_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<ProfileForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需设置个人资料&type=error").into_response();
    }

    // 先获取现有 profile，只更新用户填写的字段
    let mut profile = match state.user_service.get_profile(uid).await {
        Ok(p) => p,
        Err(e) => return Redirect::to(&format!("/profile?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    };

    // 更新字段
    if !form.name.trim().is_empty() {
        profile.name = Some(form.name.trim().to_string());
    }
    if form.gender == "M" || form.gender == "F" {
        profile.gender = Some(form.gender);
    }
    if !form.birth_date.trim().is_empty() {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(form.birth_date.trim(), "%Y-%m-%d") {
            profile.birth_date = Some(date);
        }
    }

    match state.user_service.modify_profile(&profile).await {
        Ok(_) => Redirect::to("/profile?msg=资料保存成功！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/profile?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

// ============================================================
// 好友管理路由
// ============================================================

/// GET /friends —— 好友管理主页
async fn friends_page(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    let (msg, msg_type) = flash_from_query(&params);

    let friends = state.relation_service.list_friends(uid).await.unwrap_or_default();
    let requests = state.relation_service.get_pending_requests(uid).await.unwrap_or_default();

    let mut ctx = base_context(Some(uid), Some(role), &msg, &msg_type);
    ctx.insert("title", "好友管理");
    ctx.insert("friends", &friends);
    ctx.insert("requests", &requests);
    render(&state.tera, "friends.html", &ctx)
}

/// POST /friends/search —— 搜索用户
async fn search_users_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<SearchForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    let search_results = state.relation_service.search(uid, &form.query).await.unwrap_or_default();

    let friends = state.relation_service.list_friends(uid).await.unwrap_or_default();
    let requests = state.relation_service.get_pending_requests(uid).await.unwrap_or_default();

    let mut ctx = base_context(Some(uid), Some(role), "", "");
    ctx.insert("title", "好友管理");
    ctx.insert("friends", &friends);
    ctx.insert("requests", &requests);
    ctx.insert("search_results", &search_results);
    ctx.insert("search_query", &form.query);
    render(&state.tera, "friends.html", &ctx)
}

/// POST /friends/add —— 发送好友申请
async fn add_friend_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<FriendActionForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    match state.relation_service.add_friend(uid, form.target_id).await {
        Ok(_) => Redirect::to("/friends?msg=好友申请已发送！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/friends?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /friends/accept —— 接受好友申请
async fn accept_request_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<FriendActionForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    match state.relation_service.accept_request(uid, form.target_id).await {
        Ok(_) => Redirect::to("/friends?msg=已接受好友申请！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/friends?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /friends/delete —— 删除好友
async fn delete_friend_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<FriendActionForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    match state.relation_service.delete_friend(uid, form.target_id).await {
        Ok(_) => Redirect::to("/friends?msg=已删除好友！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/friends?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /groups/create —— 创建分组
async fn create_group_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<GroupForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    match state.relation_service.create_group(uid, &form.group_name).await {
        Ok(_) => Redirect::to("/friends?msg=分组创建成功！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/friends?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /groups/delete —— 删除分组
async fn delete_group_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<GroupForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    match state.relation_service.delete_group_by_name(uid, &form.group_name).await {
        Ok(_) => Redirect::to("/friends?msg=分组已删除！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/friends?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /friends/move —— 移动好友到分组
async fn move_friend_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<MoveFriendForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role == Role::Admin {
        return Redirect::to("/admin?msg=管理员无需好友功能&type=error").into_response();
    }

    let group_name = if form.group_name == "默认" { "" } else { &form.group_name };
    match state.relation_service.move_friend_to_group(uid, form.friend_id, group_name).await {
        Ok(_) => Redirect::to("/friends?msg=移动成功！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/friends?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

// ============================================================
// 管理员路由
// ============================================================

/// GET /admin —— 管理员面板
async fn admin_page(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role != Role::Admin {
        return Redirect::to("/home?msg=无管理员权限&type=error").into_response();
    }

    let (msg, msg_type) = flash_from_query(&params);

    let all_moments = state.moment_service.admin_get_all_moments(50, 0).await.unwrap_or_default();

    let mut ctx = base_context(Some(uid), Some(role), &msg, &msg_type);
    ctx.insert("title", "管理员面板");
    ctx.insert("all_moments", &all_moments);
    render(&state.tera, "admin.html", &ctx)
}

/// POST /admin/moments/delete —— 管理员删除任意动态
async fn admin_delete_moment_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<AdminMomentDeleteForm>,
) -> Response {
    let (_uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role != Role::Admin {
        return Redirect::to("/home?msg=无管理员权限&type=error").into_response();
    }

    match state.moment_service.admin_delete_moment(form.moment_id).await {
        Ok(_) => Redirect::to("/admin?msg=动态已删除！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/admin?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

/// POST /admin/users/cancel —— 管理员注销用户
async fn admin_cancel_user_handler(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<AdminCancelForm>,
) -> Response {
    let (uid, role) = match get_session(&jar) {
        Some(s) => s,
        None => return Redirect::to("/?msg=请先登录&type=error").into_response(),
    };
    if role != Role::Admin {
        return Redirect::to("/home?msg=无管理员权限&type=error").into_response();
    }

    // 防止管理员删除自己
    if form.uid == uid {
        return Redirect::to("/admin?msg=不能注销自己！&type=error").into_response();
    }

    match state.user_service.admin_cancel_user(form.uid).await {
        Ok(_) => Redirect::to("/admin?msg=用户已注销！&type=success").into_response(),
        Err(e) => Redirect::to(&format!("/admin?msg={}&type=error", urlencoding(&e.to_string()))).into_response(),
    }
}

// ============================================================
// 路由组装
// ============================================================

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // 认证
        .route("/", get(login_page))
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
        .route("/logout", get(logout_handler))
        // 首页 / 动态
        .route("/home", get(home_page))
        .route("/moments", post(post_moment_handler))
        .route("/moments/edit", post(edit_moment_handler))
        .route("/moments/delete", post(delete_moment_handler))
        .route("/comments", post(post_comment_handler))
        .route("/comments/delete", post(delete_comment_handler))
        // 个人资料
        .route("/profile", get(profile_page))
        .route("/profile", post(profile_save_handler))
        // 好友管理
        .route("/friends", get(friends_page))
        .route("/friends/search", post(search_users_handler))
        .route("/friends/add", post(add_friend_handler))
        .route("/friends/accept", post(accept_request_handler))
        .route("/friends/delete", post(delete_friend_handler))
        .route("/friends/move", post(move_friend_handler))
        .route("/groups/create", post(create_group_handler))
        .route("/groups/delete", post(delete_group_handler))
        // 管理员
        .route("/admin", get(admin_page))
        .route("/admin/moments/delete", post(admin_delete_moment_handler))
        .route("/admin/users/cancel", post(admin_cancel_user_handler))
        // 注入共享状态
        .with_state(state)
}
