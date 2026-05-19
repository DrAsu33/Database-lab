use sqlx::mysql::{MySqlPoolOptions, MySqlPool};
use tokio::io::{self, AsyncBufReadExt};

use crate::auth;

// FSM to record the state
enum AppState {
    NotLoggedIn,
    LoggedIn(i32), // save the userid
}

// 定义极其清晰的控制流语义枚举
// the handle_not_logged_in fn should return this.
#[derive(Debug, PartialEq, Eq)]
pub enum AppFlow {
    Continue,
    Shutdown,
    // 未来如果需要热更新或重启，只需加一行：Restart,
}

// 在调试阶段，最大最小链接数先设置为1，之后可以对参数进行修改。
const MAX_CONNECTION : u32 = 1;
const MIN_CONNECTION : u32 = 1;

pub struct App {
    pool: MySqlPool,
    state: AppState,
}

// The constructions about the struct App
impl App {
    // The constuctor of App. The caller is responsible for dealing with the error.
    pub async fn new() -> Result<Self, sqlx::Error>  {
        // Propagates the Error automatically and
        // needs the caller to deal with the possible error
        let pool = Self::get_pool().await?;

        Ok(App {pool: pool, state: AppState::NotLoggedIn})
    }

    // Private fn to get the connection pool with MySQL
    async fn get_pool() -> Result<MySqlPool, sqlx::Error> {
        // Makes sure the URL in .env is read.
        // in .env there should be:
        // DATABASE_URL=mysql://lab_user:lab_password@localhost:3306/lab_DB
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
        .expect("The environmental variant DATABASE_URL should be set.");

        // Initialize the connection pool.
        // The parameters of MAX(MIN)_CONNECTION are in App.rs
        MySqlPoolOptions::new()
            .max_connections(MAX_CONNECTION)
            .min_connections(MIN_CONNECTION)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&database_url)
            .await
    }
}


// Interfaces
impl App {
    // When state is NotLoggedIn, this fn shall be called
    async fn handle_not_logged_in(&mut self) -> AppFlow {
        println!("Welcome! You have to log in first to try the features.");
        println!("Press 1 for login, 2 for register, 3 for quit.");

        // initialize the async reader
        let mut reader = io::BufReader::new(tokio::io::stdin());
        // memory allocation should be outside the loop
        let mut input = String::new();

        loop {
            input.clear();

            if reader.read_line(&mut input).await.unwrap_or(0) == 0 {
                println!("Invalid input. Please try again");
                continue;
            }

            match input.trim() {
                "1" => {
                    self.login().await;
                    return AppFlow::Continue;
                }
                "2" => {
                    self.register().await;
                    return AppFlow::Continue;
                }
                "3" => {
                    println!("Quitting...");
                    return AppFlow::Shutdown;
                }
                _ => {
                    println!("Invalid input. Please try again");
                    continue;
                }

            }
        }
    }

    // fn login deals with user's input
    async fn login(&mut self) {
        println!("Trying to log in..");
        println!("Press 1 for login, 2 for quit login.");

        // initialize the io variables first.
        // Creates a "buffered async stdin reader"
        let mut reader = io::BufReader::new(io::stdin());
        let mut input = String::new();
        
        loop {
            input.clear();

            // Deals with EOF(returns Ok(0)) and I/O errors(returns Err(_), which is converted into 0)
            if reader.read_line(&mut input).await.unwrap_or(0) == 0 {
                println!("\nInvalid input. Login process aborted.");
                return;
            }

            match input.trim() {
                "1" => {
                    input.clear();

                    println!("Please input your account number.");
                    if reader.read_line(&mut input).await.unwrap_or(0) == 0 {
                        println!("Your account input is invalid. Login process aborted.");
                        return;
                    }

                    let account = match input.trim().parse::<i32>() {
                        Ok(num) => num,
                        Err(_) => {
                            println!("Your account input is invalid. Login process aborted.");
                            return;
                        }
                    };

                    input.clear();
                    println!("Please input your password.");
                    if reader.read_line(&mut input).await.unwrap_or(0) == 0 {
                        println!("Failed to read in your password. Login process aborted.");
                        return;
                    }

                    // trim() returns a &str and should be converted manually
                    let password = input.trim();

                    // Login with the data input
                    let result = auth::login_with_data(&self.pool, account, password).await;
                    match result {
                        Ok(true) => {
                            println!("Login successed. Welcome, User {}.", account);
                            // After logged in successfully, the state machine shall advance.
                            self.state = AppState::LoggedIn(account);
                        }
                        Ok(false) => {
                            println!("Login failed: Invalid account or password.");
                        }
                        Err(e) => {
                            println!("Database error during login: {:?}", e);
                        }
                    }
                    break;

                }

                "2" => {
                    return;
                }

                _ => {
                    println!("Invalid input. Please try again");
                    continue;
                }

            }
        }
    }

    // fn register deals with user's input
    async fn register(&mut self) {
        println!("You have to input your password and a user_account will be assigned for you");

        // initialize the async reader
        let mut reader = io::BufReader::new(tokio::io::stdin());
        // memory allocation should be outside the loop
        let mut input = String::new();

        loop
        {
            println!("Press 1 for continue, 2 for quit.");
            input.clear();

            if reader.read_line(&mut input).await.unwrap_or(0) == 0 {
                println!("Invalid input. Registration process aborted.");
                return;
            }

            match input.trim() { 
                "1" => {
                    println!("Please input your password.");
                    input.clear();
                    if reader.read_line(&mut input).await.unwrap_or(0) == 0 {
                        println!("Invalid input. Registration process aborted.");
                        return;
                    }

                    let result = auth::register_with_password(&self.pool, input.trim()).await;
                    match result {
                        Ok(assigned_id) => {
                            println!("Registration success! Your assigned id is {}", assigned_id);
                            return;
                        },
                        Err(e) => {
                            println!("There's something wrong with the database. Please try again.\nError : {:?}", e);
                            continue;
                        }
                    }
                },

                "2" => {
                    return;
                }

                _ => {
                    println!("Invalid input. Please try again");
                    continue;
                }
            }
        }
    }

    async fn handle_logged_in(&mut self, user_id: i32) {
    
    }

    pub async fn run(&mut self) {
        loop {
            match self.state {
                AppState::NotLoggedIn => {
                    if self.handle_not_logged_in().await == AppFlow::Shutdown {
                        return;
                    }
                }
                AppState::LoggedIn(user_id) => {
                    self.handle_logged_in(user_id).await;
                }
            }
        }
    }
}