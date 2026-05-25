use tokio::io::{self, AsyncBufReadExt, BufReader, Stdin};
use crate::service::service::{MomentService, RelationService};
use crate::service::{UserService};
use crate::domain::DomainError;
use chrono::Local;

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
}

impl CliApplication {
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
                                        // status: 255是无关系, 0是已申请, 2是已经是好友
                                        let relation_text = match u.status {
                                            255 => "Stranger",
                                            0 => "Request Sent",
                                            2 => "Already Friends",
                                            _ => {"Unknown Status (this is a serious database error)"},
                                        };
                                        println!("ID: {} | name: {} | status: {}", u.id, u.name, relation_text);
                                    }
                                }
                            }
                            Err(e) => println!("The search failed: {}", e),
                        }
                    }
                }
                "3" => {
                    // 3. Send Friend Request
                    println!("Enter the target user ID to add:");
                    if let Ok(id_str) = self.read_next_line().await {
                        if let Ok(target_id) = id_str.parse::<u64>() {
                            match self.relation_service.add_friend(uid, target_id).await {
                                Ok(_) => println!("Friend request sent successfully!"),
                                Err(e) => println!("Failed to send request: {}", e), 
                            }
                        } else {
                            println!("Invalid ID format.");
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
                                println!("Enter the user ID to accept (enter 0 to cancel):");
                                if let Ok(id_str) = self.read_next_line().await {
                                    if let Ok(applicant_id) = id_str.parse::<u64>() {
                                        if applicant_id == 0 { continue; }
                                        match self.relation_service.accept_request(uid, applicant_id).await {
                                            Ok(_) => println!("Friend request accepted!"),
                                            Err(e) => println!("Failed to process request: {}", e),
                                        }
                                    }
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
                        } else {
                            println!("Invalid ID format.");
                        }
                    }
                }
                "6" => {
                    println!("Enter the new group name:");
                    if let Ok(name) = self.read_next_line().await {
                        match self.relation_service.create_group(uid, &name).await {
                            Ok(id) => println!("Group '{}' created successfully. Assigned ID: {}", name, id),
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
                    println!("Enter the friend ID to move:");
                    if let Ok(id_str) = self.read_next_line().await {
                        if let Ok(friend_id) = id_str.parse::<u64>() {
                            println!("Enter the target group name (press Enter to remove from current group):");
                            if let Ok(group_name) = self.read_next_line().await {
                                match self.relation_service.move_friend_to_group(uid, friend_id, &group_name).await {
                                    Ok(_) => println!("Friend group updated successfully."),
                                    Err(e) => println!("Failed to move friend: {}", e),
                                }
                            }
                        } else {
                            println!("Invalid ID format.");
                        }
                    }
                }
                _ => println!("Invalid input please try again."),
            }
        }
        Ok(())
    }

    async fn moments_loop(&mut self, uid: u64) -> Result<(), io::Error> {
        loop {
            println!("\n=== Moments & Comments ===");
            println!("1. View Moments");
            println!("2. Create New Moment");
            println!("3. Edit Moment");
            println!("4. Delete Moment");
            println!("5. Add Comment");
            println!("6. Delete Comment");
            println!("7. Return to Main Menu");

            let choice = self.read_next_line().await?;

            match choice.as_str() {
                "1" => {
                    match self.moment_service.get_friends_moments(uid).await {
                        Ok(moments) => {
                            if moments.is_empty() {
                                println!("There are no moments now visible.");
                            } else {
                                println!("\n================ Moments ================");
                                for m in moments {
                                    let edited_tag = m.is_edited();
                                    let time_str = if edited_tag {m.last_modified_time.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()}
                                    else {m.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()};
                                    if m.is_edited() {
                                        println!("\n[ID: {}] By {} Last edited at: {}", m.moment_id, m.author_name.unwrap_or_default(), time_str);
                                    }
                                    else {
                                        println!("\n[ID: {}] By {} Posted at: {}", m.moment_id, m.author_name.unwrap_or_default(), time_str);
                                    }
                                    println!("  {}", m.content);
                                    
                                    // 解包 sqlx::types::Json
                                    let comments = &m.comments.0; 
                                    if !comments.is_empty() {
                                        println!("  --- Comments ---");
                                        for c in comments {
                                            let c_time = c.created_at.with_timezone(&Local).format("%m-%d %H:%M").to_string();
                                            println!("    -> [comment ID: {}] {}: {} (commented at: {})", 
                                                c.comment_id, c.commenter_name, c.comment, c_time);
                                        }
                                    }
                                    println!("----------------------------------------");
                                }
                            }
                        }
                        Err(e) => println!("Failed to fetch moments: {}", e),
                    }
                }
                "2" => {
                    println!("Enter moment content (max 150 characters):");
                    let content = self.read_next_line().await?; 
                    match self.moment_service.post_moment(uid, &content).await {
                        Ok(id) => println!("Posted successfully! Moment ID: {}", id),
                        Err(e) => println!("Failed to post: {}", e),
                    }
                }
                "3" => {
                    println!("Enter the ID of the moment to edit:");
                    let id_str = self.read_next_line().await?;
                    if let Ok(moment_id) = id_str.parse::<u64>() {
                        println!("Enter the new content:");
                        let new_content = self.read_next_line().await?;
                        match self.moment_service.update_moment(uid, moment_id, &new_content).await {
                            Ok(_) => println!("Updated successfully!"),
                            Err(e) => println!("Update failed: {}", e),
                        }
                    }
                    else {
                        println!("Invalid input. Please try again.");
                    }
                }
                "4" => {
                    println!("Enter the moment ID to delete:");
                    let id_str = self.read_next_line().await?;
                    if let Ok(moment_id) = id_str.parse::<u64>() {
                        match self.moment_service.delete_moment(uid, moment_id).await {
                            Ok(_) => println!("Deleted successfully! Related comments have been cleared via database cascade."),
                            Err(e) => println!("Delete failed: {}", e),
                        }
                    }
                    else {
                        println!("Invalid input. Please try again.");
                    }
                }
                "5" => {
                    println!("Enter the target moment ID:");
                    let id_str = self.read_next_line().await?;
                    if let Ok(moment_id) = id_str.parse::<u64>() {
                        println!("Enter comment content (max 50 characters):");
                        let content = self.read_next_line().await?;
                        match self.moment_service.post_comment(uid, moment_id, &content).await {
                            Ok(id) => println!("Comment posted! Comment ID: {}", id),
                            Err(e) => println!("Comment failed: {}", e),
                        }
                    }
                    else {
                        println!("Invalid input. Please try again.");
                    }
                }
                "6" => {
                    println!("Enter the comment ID to delete:");
                    let id_str = self.read_next_line().await?;
                    if let Ok(comment_id) = id_str.parse::<u64>() {
                        match self.moment_service.delete_comment(uid, comment_id).await {
                            Ok(_) => println!("Comment deleted."),
                            Err(e) => println!("Delete failed: {}", e),
                        }
                    }
                }
                "7" => break,
                _ => println!("Invalid choice."),
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
                match self.user_service.verify_login(account, &password).await {
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
                self.moments_loop(user_id).await?;
            }
            _ => {
                println!("Your input is invalid, Please try again.");
            }
        }
        Ok(())
    }
}