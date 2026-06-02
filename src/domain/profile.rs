use sqlx::{FromRow, Type};
use chrono::NaiveDate;

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy, Type)]
pub enum Role {
    User = 0,
    Admin = 1,
}

impl From<u8> for Role {
    fn from(value: u8) -> Self {
        match value {
            0 => Role::User,
            1 => Role::Admin,
            _ => panic!("Invalid role value: {}", value),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct UserProfile {
    pub id: u64,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub role: Role
}

impl UserProfile {
    pub fn display(&self) {
        println!("1. Name: {}", self.name.as_deref().unwrap_or("Not set (NULL)"));
        println!("2. Gender: {}", self.gender.as_deref().unwrap_or("Not set (NULL)"));
        println!("3. Birth Date: {}", 
            self.birth_date.map(|d| d.to_string()).unwrap_or_else(|| "Not set (NULL)".to_string())
        );
    }
}