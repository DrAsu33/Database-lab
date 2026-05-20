use tokio::io::{self, AsyncBufReadExt, BufReader, Stdin};
use crate::service::UserService;
use crate::errors::DomainError;
use sqlx::mysql::{MySqlPoolOptions, MySqlPool};

// The FSM for the client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliState {
    Guest,
    Authenticated(u64), // the user's id is recorded here.
    Quit,
}

// The memory allocated here is always reused.
pub struct CliApplication {
    reader: BufReader<Stdin>,
    service: UserService,
    state: CliState,
    input_buffer: String,
}

// 在调试阶段，最大最小链接数先设置为1，之后可以对参数进行修改。
const MAX_CONNECTION : u32 = 1;
const MIN_CONNECTION : u32 = 1;

impl CliApplication {
    pub async fn new() -> Result<Self, sqlx::Error> {
        // Propagates the Error automatically and
        // needs the caller to deal with the possible error
        let pool = Self::get_pool().await?;

        Ok(Self {
            reader: BufReader::new(io::stdin()),
            service : UserService::new(pool),
            state: CliState::Guest,
            input_buffer: String::with_capacity(128),
        })
    }

    // fn to get the connection pool with MySQL
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

impl CliApplication {
    async fn read_next_line(&mut self) -> Result<&str, io::Error> {
        self.input_buffer.clear();
        if self.reader.read_line(&mut self.input_buffer).await? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
        }
        Ok(self.input_buffer.trim())
    }
}

impl CliApplication {
    pub async fn run(&mut self) -> Result<(), io::Error> {
        loop {
            match self.state {
                CliState::Guest => self.guest_loop().await?,
                CliState::Authenticated(uid) => self.auth_loop(uid).await?,
                CliState::Quit => break,
            }
        }
        Ok(())
    }

    async fn guest_loop(&mut self) -> Result<(), io::Error> {
        println!("==========================================================");
        println!("Welcome! You have to log in first to try the features.");
        println!("Press 1 for login, 2 for register, 3 for quit.");

        let choice = self.read_next_line().await?;
        match choice {
            "1" => {
                println!("Please input your account number.");
                let account = match self.read_next_line().await?.parse::<u64>() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("Your input is invalid. Please try again.");
                        return Ok(());
                    }
                };

                println!("Please input your password.");
                let password = &self.read_next_line().await?.to_string();
                match self.service.verify_login(account, &password).await {
                    Ok(_) => {
                        println!("Login successful. Welcome User {}", account);
                        self.state = CliState::Authenticated(account);
                    }
                    Err(DomainError::InvalidCredentials) => {
                        println!("Error: Incorrect ID or password.");
                    }
                    Err(e) => {
                        eprintln!("Database/System Error during login: {}", e);
                    }
                }
            }

            "2" => {
                println!("You have to input your password and a user_account will be assigned for you");
                let password = self.read_next_line().await?.to_string();

                if password.is_empty() {
                    println!("Error: Password cannot be empty.");
                    return Ok(());
                }
            
                match self.service.create_user(&password).await {
                    Ok(id) => {
                        println!("Registration succeeded! Please remember that your assigned ID is {}.", id)
                    },
                    Err(e) => {
                        eprintln!("System Error: {}", e);
                    }, 
                };
            }

            "3" => {
                println!("Trying to quit...");
                self.state = CliState::Quit;
            }

            _ => {
                println!("Your input is invalid, Please try again.");
            }
        }
        Ok(())
    }

    async fn auth_loop(&mut self, user_id: u64) -> Result<(), io::Error> {
        println!("==========================================================");
        println!("User {}, welcome! You can try the following features.", user_id);
        println!("Press 1 for logout, 2 for quit. Other functions are under development.");

        let choice = self.read_next_line().await?;
        match choice {
            "1" => {
                println!("Logging out...");
                self.state = CliState::Guest;
            }

            "2" => {
                println!("Trying to quit...");
                self.state = CliState::Quit;
            }

            _ => {
                println!("Your input is invalid, Please try again.");
            }
        }
        Ok(())
    }
}