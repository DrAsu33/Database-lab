pub mod service;
pub use service::UserService;
pub use service::RelationService;

use async_trait::async_trait;
use crate::domain::{DomainError, UserProfile, FriendItem};

// The service layer needs a struct satisfying the following trait
#[async_trait]
pub trait UserRepository {
    async fn fetch_profile(&self, uid: u64) -> Result<UserProfile, DomainError>;
    async fn save_profile(&self, profile: &UserProfile) -> Result<(), DomainError>;
    async fn login_with_data(&self, account: u64, password: &str) -> Result<bool, DomainError>;
    async fn register_with_password(&self, raw_password: &str) -> Result<u64, DomainError>;
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