pub mod service;
pub use service::UserService;
pub use service::RelationService;
pub use service::MomentService;

use async_trait::async_trait;
use crate::domain::{Role, DomainError, UserProfile, FriendItem, MomentItem};

// The service layer needs a struct satisfying the following trait
#[async_trait]
pub trait UserRepository {
    async fn fetch_profile(&self, uid: u64) -> Result<UserProfile, DomainError>;
    async fn save_profile(&self, profile: &UserProfile) -> Result<(), DomainError>;
    async fn login_with_data(&self, account: u64, password: &str) -> Result<Role, DomainError>;
    async fn register_with_password(&self, raw_password: &str) -> Result<u64, DomainError>;
    async fn force_cancel_user(&self, uid: u64) -> Result<(), DomainError>;
}

#[async_trait]
pub trait RelationRepository: Send + Sync {
    async fn search_users(&self, current_user_id: u64, query: &str) -> Result<Vec<FriendItem>, DomainError>;
    async fn send_request(&self, user_id: u64, target_id: u64) -> Result<(), DomainError>;
    async fn accept_request(&self, user_id: u64, applicant_id: u64) -> Result<(), DomainError>;
    async fn remove_friend(&self, user_id: u64, friend_id: u64) -> Result<(), DomainError>;
    async fn list_relations(&self, user_id: u64, status: u8) -> Result<Vec<FriendItem>, DomainError>;
    async fn list_pending_requests(&self, my_user_id: u64) -> Result<Vec<FriendItem>, DomainError>;

    async fn create_group(&self, user_id: u64, group_name: &str) -> Result<u64, DomainError>;
    async fn delete_group(&self, user_id: u64, group_id: u64) -> Result<(), DomainError>;
    async fn get_group_id_by_name(&self, user_id: u64, group_name: &str) -> Result<Option<u64>, DomainError>;
    async fn set_friend_group(&self, user_id: u64, friend_id: u64, group_id: Option<u64>) -> Result<(), DomainError>;
}

#[async_trait]
pub trait MomentRepository: Send + Sync {
    async fn fetch_friends_moments(&self, user_id: u64, limit: u32, offset: u32) -> Result<Vec<MomentItem>, DomainError>;
    async fn post_moment(&self, uid: u64, content: &str) -> Result<u64, DomainError>;
    async fn update_moment(&self, uid: u64, moment_id: u64, new_content: &str) -> Result<(), DomainError>;
    async fn delete_moment_by_author(&self, uid: u64, moment_id: u64) -> Result<(), DomainError>;
    async fn post_comment(&self, uid: u64, moment_id:u64, content: &str) -> Result<u64, DomainError>;
    async fn delete_comment(&self, uid: u64, moment_id:u64) -> Result<(), DomainError>;
    
    async fn fetch_all_moments_for_admin(&self, limit: u32, offset: u32) -> Result<Vec<MomentItem>, DomainError>;
    async fn force_delete_moment(&self, moment_id: u64) -> Result<(), DomainError>;
}