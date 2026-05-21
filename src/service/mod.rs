pub mod service;
pub use service::UserService;

use async_trait::async_trait;
use crate::domain::{DomainError, UserProfile};

// The service layer needs a struct satisfying the following trait
#[async_trait]
pub trait UserRepository {
    async fn fetch_profile(&self, uid: u64) -> Result<UserProfile, DomainError>;
    async fn save_profile(&self, profile: &UserProfile) -> Result<(), DomainError>;
    async fn login_with_data(&self, account: u64, password: &str) -> Result<bool, DomainError>;
    async fn register_with_password(&self, raw_password: &str) -> Result<u64, DomainError>;
}