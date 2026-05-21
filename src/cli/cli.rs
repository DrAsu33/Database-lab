use tokio::io::{self, AsyncBufReadExt, BufReader, Stdin};
use crate::service::{UserService};
use crate::domain::DomainError;

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

impl CliApplication {
    pub fn new(service: UserService) -> Self {
        Self {
            reader: BufReader::new(io::stdin()),
            service,
            state: CliState::Guest,
            input_buffer: String::with_capacity(128),
        }
    }

    // In client service all the input shall be get within this fn
    async fn read_next_line(&mut self) -> Result<String, io::Error> {
        self.input_buffer.clear();
        if self.reader.read_line(&mut self.input_buffer).await? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
        }
        Ok(self.input_buffer.trim().to_string())
    }
}

impl CliApplication {
    // After the user choosed to edit his profile, the fn is called in the auth_loop
    async fn profile_edit_loop(&mut self, uid: u64) -> Result<(), io::Error> {
        loop {
            let mut profile = match self.service.get_profile(uid).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to fetch profile: {}", e);
                    return Ok(()); // 抓取失败，退回上一级菜单
                }
            };

            println!("--- User Profile ---");
            profile.display();
            println!("Enter the number of the field to edit, or 0 to return:");
            let choice = self.read_next_line().await?;

            match choice.as_str() {
                "1" => {
                    println!("Enter new name:");
                    let new_name = self.read_next_line().await?;
                    if !new_name.is_empty() {
                        profile.name = Some(new_name);
                    }
                }
                "2" => {
                    println!("Enter new gender (M/F):");
                    let new_gender = self.read_next_line().await?;
                    // 这里可以做严格的枚举校验
                    if new_gender == "M" || new_gender == "F" {
                        profile.gender = Some(new_gender);
                    } else {
                        println!("Invalid input. Only M or F allowed.");
                        continue;
                    }
                }
                "3" => {
                    println!("Enter new birth date (YYYY-MM-DD):");
                    let date_str = self.read_next_line().await?;
                    match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(date) => profile.birth_date = Some(date),
                        Err(_) => {
                            println!("Invalid date format. Please use YYYY-MM-DD.");
                            continue;
                        }
                    }
                }
                "0" => {
                    break;
                }
                _ => {
                    println!("Invalid selection.");
                    continue; // skip this loop
                }
            }

            match self.service.modify_profile(&profile).await {
                Ok(_) => {
                    println!("Modification success!");
                },
                Err(e) => {
                    eprintln!("Failed to modify profile: {}", e);
                    return Ok(()); // 抓取失败，退回上一级菜单
                }
            }
        }

    Ok(())
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
        match choice.as_str() {
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
                let password = self.read_next_line().await?;
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
                let password = self.read_next_line().await?;

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
        println!("Press 1 for logout, 2 for quit, 3 for modifying your personal information. Other functions are under development.");

        let choice = self.read_next_line().await?;
        match choice.as_str() {
            "1" => {
                println!("Logging out...");
                self.state = CliState::Guest;
            }

            "2" => {
                println!("Trying to quit...");
                self.state = CliState::Quit;
            }

            "3" => {
                self.profile_edit_loop(user_id).await?;
            }

            _ => {
                println!("Your input is invalid, Please try again.");
            }
        }
        Ok(())
    }
}