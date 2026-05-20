use sqlx::MySqlPool;
use crate::auth::{login_with_data, register_with_password};
use crate::errors::DomainError;

// The service struct only have the underlying resources and does not have a state
#[derive(Clone)]
pub struct UserService {
    pool: MySqlPool,
}

impl UserService {
    // The constuctor of UserService. Needs a pool.
    // After the construction, the ownership is transmitted
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    // A pure verify fn Returns () if success, errors if failed
    pub async fn verify_login(&self, account: u64, password: &str) -> Result<(), DomainError> {
        let is_valid = login_with_data(&self.pool, account, password).await
            .map_err(DomainError::Database)?;
            
        if !is_valid {
            return Err(DomainError::InvalidCredentials);
        }
        
        Ok(())
    }

    pub async fn create_user(&self, password: &str) -> Result<u64, DomainError> {
        let id = register_with_password(&self.pool, password).await?;
        Ok(id)
    }
}