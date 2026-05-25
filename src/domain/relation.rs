use sqlx::FromRow;

// 用于返回搜索结果和好友列表
#[derive(Debug, FromRow)]
pub struct FriendItem {
    pub id: u64,
    pub name: String,
    pub status: u8, // 搜索时可以通过状态区分是陌生人、已申请还是已经是好友
    pub group_name: Option<String>, //允许为 Option。如果是 NULL，说明该好友处于“默认/未分组”状态
}