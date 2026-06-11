use sqlx::{FromRow, Type};
use chrono::NaiveDate;

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy, Type)]
pub enum Role {
    User = 0,
    Admin = 1,
}

#[derive(Debug, FromRow)]
pub struct UserProfile {
    pub id: u64,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub age: Option<u32>,
}

impl UserProfile {
    pub fn display(&self) {
        println!("1. Name: {}", self.name.as_deref().unwrap_or("Not set (NULL)"));
        println!("2. Gender: {}", self.gender.as_deref().unwrap_or("Not set (NULL)"));
        println!("3. Birth Date: {}", 
            self.birth_date.map(|d| d.to_string()).unwrap_or_else(|| "Not set (NULL)".to_string())
        );
        println!("   Age: {}", 
            self.age.map(|d| d.to_string()).unwrap_or_else(|| "Not set (NULL)".to_string())
        );
    }
}