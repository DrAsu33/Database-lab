use tokio::io::{self, AsyncBufReadExt, BufReader, Stdin};
use crate::infrastructure::{COMMENT_LIMIT, MOMENT_LIMIT};
use crate::service::service::{MomentService, RelationService};
use crate::service::{UserService};
use crate::domain::{DomainError, Role};

// The FSM for the client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliState {
    Guest,
    UserAuthenticated(u64),
    AdminAuthenticated(u64),
    Quit,
}

// The memory allocated here is always reused.
pub struct CliApplication {
    reader: BufReader<Stdin>,
    user_service: UserService,
    relation_service: RelationService,
    moment_service: MomentService,
    state: CliState,
    input_buffer: String,
}

impl CliApplication {
    pub fn new(user_service: UserService, relation_service: RelationService, moment_service: MomentService) -> Self {
        Self {
            reader: BufReader::new(io::stdin()),
            user_service,
            relation_service,
            moment_service,
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

    // print out the prompt and try to parse the u64.
    async fn read_u64_prompt(&mut self, prompt: &str) -> Option<u64> {
        println!("{}", prompt);
        if let Ok(input) = self.read_next_line().await {
            if let Ok(id) = input.parse::<u64>() {
                return Some(id);
            }
        }
        println!("Invalid input format. Must be a positive integer.");
        None
    }
}

impl CliApplication {
    const PAGE_SIZE: u32 = 5;

    // After the user choosed to edit his profile, the fn is called in the auth_loop
    async fn profile_edit_loop(&mut self, uid: u64) -> Result<(), io::Error> {
        loop {
            let mut profile = match self.user_service.get_profile(uid).await {
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

            match self.user_service.modify_profile(&profile).await {
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

    async fn friends_loop(&mut self, uid: u64) -> Result<(), io::Error> {
        loop {
            println!("\n=== Friend Management ===");
            println!("Note: One friend can have at most 1 group and groups' name should be different.");
            println!("0. Back to Main Menu");
            println!("1. View My Friends");
            println!("2. Search Users");
            println!("3. Send Friend Request");
            println!("4. Manage Friend Requests (Pending)");
            println!("5. Remove Friend");
            println!("6. Create Group");
            println!("7. Delete Group");
            println!("8. Move Friend");
            
            let choice = self.read_next_line().await?;

            match choice.as_str() {
                "0" => break, // 0. Back to Main Menu
                "1" => {
                    // 1. View My Friends
                    match self.relation_service.list_friends(uid).await {
                        Ok(friends) => {
                            if friends.is_empty() {
                                println!("You do not have any friends now.");
                            } else {
                                println!("\n--- Your friends list ---");
                                for f in friends {
                                    let group_display = f.group_name.as_deref().unwrap_or("默认分组");
                                    println!("ID: {} | name: {} | group: [{}]", f.id, f.name, group_display);
                                }
                            }
                        }
                        Err(e) => println!("获取好友失败: {}", e),
                    }
                }
                "2" => {
                    // 2. Search Users
                    println!("Please input the name prefix:");
                    if let Ok(query) = self.read_next_line().await {
                        match self.relation_service.search(uid, &query).await {
                            Ok(users) => {
                                if users.is_empty() {
                                    println!("Failed to find such users.");
                                } else {
                                    println!("\n--- Results ---");
                                    for u in users {
                                        println!("ID: {} | name: {} | status: {}", u.id, u.name, u.status);
                                    }
                                }
                            }
                            Err(e) => println!("The search failed: {}", e),
                        }
                    }
                }
                "3" => {
                    // 3. Send Friend Request
                    if let Some(target_id) = self.read_u64_prompt("Enter the target user ID to add:").await {
                        match self.relation_service.add_friend(uid, target_id).await {
                            Ok(_) => println!("Friend request sent successfully!"),
                            Err(e) => println!("Failed to send request: {}", e), 
                        }
                    }
                }
                "4" => {
                    // 4. Manage Friend Requests (Pending)
                    match self.relation_service.get_pending_requests(uid).await {
                        Ok(requests) => {
                            if requests.is_empty() {
                                println!("No pending friend requests.");
                            } else {
                                println!("\n--- Pending Requests ---");
                                for req in requests {
                                    println!("ID: {} | From: {}", req.id, req.name);
                                }
                                if let Some(applicant_id) = self.read_u64_prompt("Enter the user ID to accept (enter 0 to cancel):").await {
                                    if applicant_id == 0 { continue; }
                                    match self.relation_service.accept_request(uid, applicant_id).await {
                                        Ok(_) => println!("Friend request accepted!"),
                                        Err(e) => println!("Failed to process request: {}", e),
                                    }
                                } else {
                                    println!("Invalid ID format.");
                                }
                            }
                        }
                        Err(e) => println!("Failed to fetch requests: {}", e),
                    }
                }
                "5" => {
                    // 5. Remove Friend
                    println!("Enter the friend ID to remove:");
                    if let Ok(id_str) = self.read_next_line().await {
                        if let Ok(friend_id) = id_str.parse::<u64>() {
                            match self.relation_service.delete_friend(uid, friend_id).await {
                                Ok(_) => println!("Friend or request removed."),
                                Err(e) => println!("Failed to remove: {}", e),
                            }
                        }
                    }
                }
                "6" => {
                    println!("Enter the new group name:");
                    if let Ok(name) = self.read_next_line().await {
                        match self.relation_service.create_group(uid, &name).await {
                            Ok(_) => println!("Group '{}' created successfully.", name),
                            Err(e) => println!("Failed to create group: {}", e),
                        }
                    }
                }
                "7" => {
                    println!("Enter the group name to delete (friends in this group will be unassigned):");
                    if let Ok(name) = self.read_next_line().await {
                        match self.relation_service.delete_group_by_name(uid, &name).await {
                            Ok(_) => println!("Group '{}' deleted successfully.", name),
                            Err(e) => println!("Failed to delete group: {}", e),
                        }
                    }
                }
                "8" => {
                    if let Some(friend_id) = self.read_u64_prompt("Enter the friend ID to move:").await {
                        println!("Enter the target group name (press Enter to remove from current group):");
                        let group_name = self.read_next_line().await?;
                        match self.relation_service.move_friend_to_group(uid, friend_id, &group_name).await {
                            Ok(_) => println!("Friend group updated successfully."),
                            Err(e) => println!("Failed to move friend: {}", e),
                        }
                    }
                }
                _ => println!("Invalid input please try again."),
            }
        }
        Ok(())
    }

    async fn view_moments_loop(&mut self, uid: u64) -> Result<(), io::Error> {
        let mut current_offset: u32 = 0;
        let mut has_next_page: bool = false;

        loop {
            println!("\n\n=======================================================");
            println!("             Moments (Page {} )", (current_offset / Self::PAGE_SIZE) + 1);
            println!("=======================================================");

            // Fetch data based on the current offset
            match self.moment_service.get_friends_moments(uid, Self::PAGE_SIZE, current_offset).await {
                Ok(moments) => {
                    has_next_page = moments.len() as u32 == Self::PAGE_SIZE;
                    if moments.is_empty() {
                        println!("\n There's no data in this page since it's the last page.");
                    } else {
                        for m in &moments {
                            m.display();
                        }
                    }
                }
                Err(e) => println!("Data fetch failed: {}", e),
            }

            println!("\nOperands:");
            println!("  [n] next page | [p] previous page");
            println!("  [post] post your new comment");
            println!("  [edit <id>] edit the specific moment.(note: you have to be the author yourself.)");
            println!("  [del <id>] delete the specific moment.(note: you have to be the author yourself.)");
            println!("  [cmt <id>] comment on the specific moment.");
            println!("  [del_cmt <id>] delete the specifc comment");
            println!("  [q] quit to main menu.");
            println!("Please input your command: ");

            let choice = self.read_next_line().await?;
            let parts: Vec<&str> = choice.split_whitespace().collect();
            if parts.is_empty() { continue; }

            // An assistance closure to parse the ID safely.
            // Returns Some(id) if there is an ID and None if there's none
            let parse_id = || -> Option<u64> {
                parts.get(1).and_then(|s| s.parse::<u64>().ok())
            };

            match parts[0] {
                "n" => {
                    if has_next_page {
                        current_offset += Self::PAGE_SIZE;
                    } else {
                        println!("\nYou are already on the last page. Cannot go further.");
                    }
                }
                "p" => {
                    // Check whether the offset can be minused, or it'll panic
                    if current_offset >= Self::PAGE_SIZE {
                        current_offset -= Self::PAGE_SIZE;
                    } else {
                        println!("\nYou can't go to the previous page since it's the first page.");
                    }
                }
                "post" => {
                    println!("Enter moment content (max {} characters):", MOMENT_LIMIT);
                    let content = self.read_next_line().await?; 
                    match self.moment_service.post_moment(uid, &content).await {
                        Ok(id) => println!("[Success] Posted successfully! Moment ID: {}", id),
                        Err(e) => println!("[Failed] Failed to post: {}", e),
                    }
                }
                "edit" => {
                    if let Some(moment_id) = parse_id() {
                        println!("Enter the new content:");
                        let new_content = self.read_next_line().await?;
                        match self.moment_service.update_moment(uid, moment_id, &new_content).await {
                            Ok(_) => println!("[Success] Updated successfully!"),
                            Err(e) => println!("[Failed] Update failed: {}", e),
                        }
                    } else {
                        println!("\n[Error] Invalid ID format. Usage: edit <id>");
                    }
                }
                "del" => {
                    if let Some(moment_id) = parse_id() {
                        match self.moment_service.delete_moment(uid, moment_id).await {
                            Ok(_) => println!("[Success] Deleted successfully! Related comments have been cleared via database cascade."),
                            Err(e) => println!("[Failed] Delete failed: {}", e),
                        }
                    }
                    else {
                        println!("\n[Error] Invalid ID format. Usage: del <id>");
                    }
                } 
                "cmt" => {
                    if let Some(moment_id) = parse_id() {
                        println!("Enter comment content (max {} chars):", COMMENT_LIMIT);
                        let content = self.read_next_line().await?;
                        match self.moment_service.post_comment(uid, moment_id, &content).await {
                            Ok(id) => println!("[Success] Comment posted! Comment ID: {}", id),
                            Err(e) => println!("[Failed] Comment failed: {}", e),
                        }
                    } else {
                        println!("\n[Error] Invalid ID format. Usage: cmt <id>");
                    }
                }
                "del_cmt" => {
                    if let Some(comment_id) = parse_id() {
                        match self.moment_service.delete_comment(uid, comment_id).await {
                            Ok(_) => println!("[Success] Comment deleted."),
                            Err(e) => println!("[Failed] Delete failed: {}", e),
                        }
                    } else {
                        println!("\n[Error] Invalid ID format. Usage: del_cmt <id>");
                    }
                } 
                "q" => {
                    println!("Quitting to the main menu...");
                    break;
                }
                _ => println!("\n[Error] Invalid input. Please try again."),
            }
        }
        Ok(())
    }

    async fn admin_audit_loop(&mut self) -> Result<(), io::Error> {
        let mut current_offset: u32 = 0;
        let mut has_next_page: bool = false;

        loop {
            println!("\n\n=======================================================");
            println!("             Audit center (Page {} )", (current_offset / Self::PAGE_SIZE) + 1);
            println!("=======================================================");

            // Fetch data based on the current offset
            match self.moment_service.admin_get_all_moments(Self::PAGE_SIZE, current_offset).await {
                Ok(moments) => {
                    has_next_page = moments.len() as u32 == Self::PAGE_SIZE;
                    if moments.is_empty() {
                        println!("\n There's no data in this page since it's the last page.");
                    } else {
                        for m in &moments {
                            m.display();
                        }
                    }
                }
                Err(e) => println!("Data fetch failed: {}", e),
            }

            println!("\nOperands:");
            println!("  [n] next page | [p] previous page");
            println!("  [d <id>] delete violating moment. (e.g. d 1024)");
            println!("  [q] quit the auditing center.");
            println!("Please input your command: ");

            let choice = self.read_next_line().await?;
            let parts: Vec<&str> = choice.split_whitespace().collect();
            if parts.is_empty() { continue; }

            match parts[0] {
                "n" => {
                    if has_next_page {
                        current_offset += Self::PAGE_SIZE;
                    } else {
                        println!("\nYou are already on the last page. Cannot go further.");
                    }
                }
                "p" => {
                    // Check whether the offset can be minused, or it'll panic
                    if current_offset >= Self::PAGE_SIZE {
                        current_offset -= Self::PAGE_SIZE;
                    } else {
                        println!("\nYou can't go to the previous page since it's the first page.");
                    }
                }
                "d" => {
                    // parse the ID and cancel it
                    if parts.len() == 2 {
                        if let Ok(target_id) = parts[1].parse::<u64>() {
                            match self.moment_service.admin_delete_moment(target_id).await {
                                Ok(_) => println!("\n[Success] Violating moment (ID: {}) has been cleared!", target_id),
                                Err(e) => println!("\n[Fail] Deletion anomaly: {}", e),
                            }
                        } else {
                            println!("\n[Fail] ID format is incorrect.");
                        }
                    } else {
                        println!("\n[Fail] ID missing. Note: d <id>");
                    }
                }
                "q" => {
                    println!("Quitting the audit center...");
                    break;
                }
                _ => println!("\n[Fail] Invalid input. Please try again."),
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
                CliState::UserAuthenticated(uid) => self.auth_loop(uid).await?,
                CliState::AdminAuthenticated(uid) => self.admin_auth_loop(uid).await?,
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
                let account = match self.read_u64_prompt("Please input your account number.").await {
                    Some(num) => num,
                    None => {
                        println!("Your input is invalid. Please try again.");
                        return Ok(());
                    }
                };

                println!("Please input your password.");
                let password = self.read_next_line().await?;
                match self.user_service.verify_login(account, &password).await {
                    Ok(Role::User) => {
                        println!("Login successful. Welcome User {}", account);
                        self.state = CliState::UserAuthenticated(account);
                    }
                    Ok(Role::Admin) => {
                        println!("Login successful. Welcome Admin {}", account);
                        self.state = CliState::AdminAuthenticated(account);
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
            
                match self.user_service.create_user(&password).await {
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
        println!("Press 1 for logout, 2 for quit, 3 for modifying your personal information.");
        println!("4 for friends, 5 for checking moments.");
        println!("Other functions are under development.");

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
            "4" => {
                self.friends_loop(user_id).await?;
            }
            "5" => {
                self.view_moments_loop(user_id).await?;
            }
            _ => {
                println!("Your input is invalid, Please try again.");
            }
        }
        Ok(())
    }

    async fn admin_auth_loop(&mut self, user_id: u64) -> Result<(), io::Error> {
        println!("==========================================================");
        println!("Admin {}, welcome!", user_id);
        println!("Press 1 for logout, 2 for quit, 3 for modifying your personal information.");
        println!("4 for cancel users, 5 for auditing moments.");
        println!("Other functions are under development.");

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
            "4" => {
                if let Some(target_id) = self.read_u64_prompt("Input the id of the user you want to cancel.").await{
                    match self.user_service.admin_cancel_user(target_id).await {
                        Ok(_) => println!("User cancelled successfully!"),
                        Err(e) => println!("Failed to cancel the certain user: {}", e), 
                    }
                }
            }
            "5" => {
                self.admin_audit_loop().await?;
            }
            _ => {
                println!("Your input is invalid, Please try again.");
            }
        }
        Ok(())
    }
}