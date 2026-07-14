use sqlx::{FromRow, Type};
use chrono::NaiveDate;
use serde::Serialize;

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy, Type, Serialize)]
pub enum Role {
    User = 0,
    Admin = 1,
}

#[derive(Debug, FromRow, Serialize)]
pub struct UserProfile {
    pub id: u64,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub age: Option<u32>,
}