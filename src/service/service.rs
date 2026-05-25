use crate::domain::{DomainError, FriendItem, MomentItem, UserProfile};
use crate::service::{MomentRepository, UserRepository};
use crate::service::RelationRepository;
use crate::infrastructure::{MOMENT_LIMIT, COMMENT_LIMIT};
use std::sync::Arc;

// The service struct only have the underlying resources and does not have a state
pub struct UserService{
    /// repo：用户数据访问接口（repository abstraction）
    /// Arc<...>
    /// - 原子引用计数指针（线程安全）允许多个 service / handler / task 共享同一个 repo 实例 clone 成本低（只增加引用计数，不复制数据）
    ///
    /// dyn UserRepository
    /// - trait object（动态分发）表示“某个实现了 UserRepository 的具体类型”，但这里不关心是谁 运行时通过 vtable 调用具体实现（如 MySqlUserRepository）
    ///
    /// Send + Sync
    /// - Send：可以在线程之间安全移动- Sync：可以被多个线程同时引用
    ///
    /// trait 中的方法必须是 async + Send（配合 async_trait）
    /// Rust 会自动做 Deref coercion（Arc → &T）
    repo: Arc<dyn UserRepository + Send + Sync>
}

pub struct RelationService {
    repo: Arc<dyn RelationRepository + Send + Sync>,
}

pub struct MomentService {
    repo: Arc<dyn MomentRepository + Send + Sync>,
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

impl RelationService {
    // 构造函数接收的参数类型必须是确定的 Arc<dyn Trait>
    pub fn new(repo: Arc<dyn RelationRepository>) -> Self {
        Self { repo }
    }

    pub async fn search(&self, current_user_id: u64, query: &str) -> Result<Vec<FriendItem>, DomainError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(DomainError::SystemFailure("Query cannot be empty".into()));
        }
        self.repo.search_users(current_user_id, trimmed).await
    }

    pub async fn add_friend(&self, current_user_id: u64, target_id: u64) -> Result<(), DomainError> {
        if current_user_id == target_id {
            return Err(DomainError::InvalidRelationState);
        }
        self.repo.send_request(current_user_id, target_id).await
    }

    pub async fn accept_request(&self, user_id: u64, applicant_id: u64) -> Result<(), DomainError> {
        self.repo.accept_request(user_id, applicant_id).await
    }

    pub async fn delete_friend(&self, user_id: u64, friend_id: u64) -> Result<(), DomainError> {
        self.repo.remove_friend(user_id, friend_id).await
    }

    pub async fn list_friends(&self, user_id: u64) -> Result<Vec<FriendItem>, DomainError> {
        self.repo.list_relations(user_id, 2).await
    }

    pub async fn get_pending_requests(&self, my_user_id: u64) -> Result<Vec<FriendItem>, DomainError> {
        self.repo.list_pending_requests(my_user_id).await
    }

    pub async fn create_group(&self, current_user_id: u64, group_name: &str) -> Result<u64, DomainError> {
        let name = group_name.trim();
        if name.is_empty() {
            return Err(DomainError::SystemFailure("Group name cannot be empty".into()));
        }
        self.repo.create_group(current_user_id, name).await
    }

    pub async fn delete_group_by_name(&self, current_user_id: u64, group_name: &str) -> Result<(), DomainError> {
        let name = group_name.trim();
        let group_id = self.repo.get_group_id_by_name(current_user_id, name).await?
            .ok_or(DomainError::GroupNotFound)?; // Option -> Result

        self.repo.delete_group(current_user_id, group_id).await
    }

    pub async fn move_friend_to_group(&self, current_user_id: u64, friend_id: u64, group_name: &str) -> Result<(), DomainError> {
        let name = group_name.trim();
        // 允许传入空字符串或 "默认" 来代表将好友移出所有分组 (设为 NULL)
        let target_group_id = if name.is_empty() {
            None
        } else {
            let id = self.repo.get_group_id_by_name(current_user_id, name).await?
                .ok_or(DomainError::GroupNotFound)?;
            Some(id)
        };

        self.repo.set_friend_group(current_user_id, friend_id, target_group_id).await
    }


}

impl MomentService {
    pub fn new(repo: Arc<dyn MomentRepository>) -> Self {
        Self { repo }
    }

    fn validate_content(raw_content: &str, limit: usize) -> Result<&str, DomainError> {
        let clean = raw_content.trim();
        if clean.is_empty() {
            return Err(DomainError::EmptyContent);
        }
        if clean.chars().count() > limit {
            return Err(DomainError::CharacterLimitExceeded);
        }
        Ok(clean)
    }

    pub async fn get_friends_moments(&self, uid: u64) -> Result<Vec<MomentItem>, DomainError> {
        self.repo.fetch_friends_moments(uid).await
    }

    pub async fn post_moment(&self, uid: u64, raw_content: &str) -> Result<u64, DomainError> {
        let clean_content = Self::validate_content(raw_content, MOMENT_LIMIT)?;
        self.repo.post_moment(uid, clean_content).await
    }

    pub async fn update_moment(&self, uid: u64, moment_id: u64, raw_content: &str) -> Result<(), DomainError> {
        let clean_content = Self::validate_content(raw_content, MOMENT_LIMIT)?;
        self.repo.update_moment(uid, moment_id, clean_content).await
    }

    pub async fn delete_moment(&self, uid: u64, moment_id: u64) -> Result<(), DomainError> {
        self.repo.delete_moment(uid, moment_id).await
    }

    pub async fn post_comment(&self, uid: u64, moment_id: u64, raw_content: &str) -> Result<u64, DomainError> {
        let clean_content = Self::validate_content(raw_content, COMMENT_LIMIT)?;
        self.repo.post_comment(uid, moment_id, clean_content).await
    }

    pub async fn delete_comment(&self, uid: u64, comment_id: u64) -> Result<(), DomainError> {
        self.repo.delete_comment(uid, comment_id).await
    }
}