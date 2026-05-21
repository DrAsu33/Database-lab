use crate::domain::{DomainError, UserProfile};
use crate::service::UserRepository;
use std::sync::Arc;

// The service struct only have the underlying resources and does not have a state
pub struct UserService{
    /// repo：用户数据访问接口（repository abstraction）
    /// Arc<...>
    /// --------
    /// - 原子引用计数指针（线程安全）
    /// - 允许多个 service / handler / task 共享同一个 repo 实例
    /// - clone 成本低（只增加引用计数，不复制数据）
    ///
    /// dyn UserRepository
    /// ------------------
    /// - trait object（动态分发）
    /// - 表示“某个实现了 UserRepository 的具体类型”，但这里不关心是谁
    /// - 运行时通过 vtable 调用具体实现（如 MySqlUserRepository）
    ///
    /// Send + Sync
    /// -----------
    /// - Send：可以在线程之间安全移动
    /// - Sync：可以被多个线程同时引用
    ///
    /// trait 中的方法必须是 async + Send（配合 async_trait）
    ///
    /// Rust 会自动做 Deref coercion（Arc → &T）

    repo: Arc<dyn UserRepository + Send + Sync>
}

impl UserService {
    // The constuctor of UserService. Needs a pool.
    // After the construction, the ownership is transmitted
    pub fn new(repo: Arc<dyn UserRepository + Send + Sync>) -> Self {
        Self { repo }
    }

    // A pure verify fn Returns () if success, errors if failed
    pub async fn verify_login(&self, account: u64, password: &str) -> Result<(), DomainError> {
        let is_valid = self.repo.login_with_data(account, password).await?;
            
        if !is_valid {
            return Err(DomainError::InvalidCredentials);
        }
        
        Ok(())
    }

    // Note: The following 2 fn can convert sqlx:Error into DomainError automatically
    pub async fn create_user(&self, password: &str) -> Result<u64, DomainError> {
        // The password here shall be encrypted later
        let id = self.repo.register_with_password(password).await?;
        Ok(id)
    }

    pub async fn get_profile(&self, uid: u64) -> Result<UserProfile, DomainError> {
        let res = self.repo.fetch_profile(uid).await?;

        Ok(res)
    }

    pub async fn modify_profile(&self, profile: &UserProfile) -> Result<(), DomainError> {
        self.repo.save_profile(profile).await?;

        Ok(())
    }
}