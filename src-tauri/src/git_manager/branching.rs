use crate::database::connection::Database;
use git2::Repository;
use chrono::Utc;

/// Create a save checkpoint with user-named branch
pub async fn create_save_checkpoint(
    db: &std::sync::Arc<tokio::sync::Mutex<Database>>,
    master_repo_path: &str,
    game_id: i64, 
    save_name: &str
) -> Result<String, String> {
    crate::logger::info("GIT_BRANCHING", &format!("Creating save checkpoint for game_id: {}, save_name: {}", game_id, save_name), None);
    
    let game_name = {
        let conn_guard = db.lock().await;
        let conn = conn_guard.get_connection().await;
        
        crate::logger::debug("GIT_BRANCHING", &format!("Querying game name for game_id: {}", game_id), None);
        
        let mut stmt = conn.prepare("SELECT name FROM games WHERE id = ?")
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", "Failed to prepare game query", Some(&e.to_string()));
                format!("Failed to prepare game query: {}", e)
            })?;
        
        stmt.query_row([game_id], |row| row.get::<_, String>(0))
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", &format!("Failed to get game name for game_id: {}", game_id), Some(&e.to_string()));
                format!("Failed to get game name: {}", e)
            })?
    };

    // Create branch name: gamename-save-name
    let branch_name = format!("{}-{}", game_name, save_name);
    crate::logger::info("GIT_BRANCHING", &format!("Branch name: {}", branch_name), None);
    
    // Check if branch exists (outside git2 scope so we can use it later)
    let branch_exists = {
        let repo = Repository::open(master_repo_path)
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", "Failed to open master repository", Some(&e.to_string()));
                format!("Failed to open master repository: {}", e)
            })?;
        let exists = repo.find_branch(&branch_name, git2::BranchType::Local).is_ok();
        exists
    };
    
    // Perform git operations in a synchronous block or scope
    // Since git2 is synchronous, we can just do it here.
    // The issue is if we hold any git2 types across an await.
    // We are not calling any async functions inside the git block below.
    {
        // Open master repository
        let repo = Repository::open(master_repo_path)
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", "Failed to open master repository", Some(&e.to_string()));
                format!("Failed to open master repository: {}", e)
            })?;

        // Check if branch already exists
        if branch_exists {
            crate::logger::info("GIT_BRANCHING", &format!("Branch '{}' already exists, switching to it", branch_name), None);
            
            // Switch to existing branch
            let existing_branch = repo.find_branch(&branch_name, git2::BranchType::Local)
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", "Failed to get branch", Some(&e.to_string()));
                    format!("Failed to get branch: {}", e)
                })?;
            
            let branch_commit = existing_branch.get().peel_to_commit()
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", "Failed to get branch commit", Some(&e.to_string()));
                    format!("Failed to get branch commit: {}", e)
                })?;

            repo.checkout_tree(&branch_commit.into_object(), None)
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", &format!("Failed to checkout branch '{}'", branch_name), Some(&e.to_string()));
                    format!("Failed to checkout branch '{}': {}", branch_name, e)
                })?;

            repo.set_head(&format!("refs/heads/{}", branch_name))
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", &format!("Failed to set HEAD to branch '{}'", branch_name), Some(&e.to_string()));
                    format!("Failed to set HEAD to branch '{}': {}", branch_name, e)
                })?;

            crate::logger::info("GIT_BRANCHING", &format!("Successfully switched to existing branch: {}", branch_name), None);
            
            // Drop git2 types before await (end of scope)
        } else {
            // Get current branch to fork from
            let current_commit = repo.head()
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", "Failed to get HEAD", Some(&e.to_string()));
                    format!("Failed to get HEAD: {}", e)
                })?
                .peel_to_commit()
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", "Failed to get current commit", Some(&e.to_string()));
                    format!("Failed to get current commit: {}", e)
                })?;

            crate::logger::debug("GIT_BRANCHING", &format!("Creating new branch '{}' from commit: {}", branch_name, current_commit.id()), None);

            // Create new branch from current HEAD
            repo.branch(&branch_name, &current_commit, false)
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", &format!("Failed to create branch '{}'", branch_name), Some(&e.to_string()));
                    format!("Failed to create branch '{}': {}", branch_name, e)
                })?;

            // Checkout the new branch
            let branch_ref = repo.find_branch(&branch_name, git2::BranchType::Local)
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", &format!("Failed to find newly created branch '{}'", branch_name), Some(&e.to_string()));
                    format!("Failed to find branch '{}': {}", branch_name, e)
                })?;

            let branch_commit = branch_ref.get().peel_to_commit()
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", "Failed to get branch commit", Some(&e.to_string()));
                    format!("Failed to get branch commit: {}", e)
                })?;

            repo.checkout_tree(&branch_commit.into_object(), None)
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", &format!("Failed to checkout branch '{}'", branch_name), Some(&e.to_string()));
                    format!("Failed to checkout branch '{}': {}", branch_name, e)
                })?;

            // Set HEAD to new branch
            repo.set_head(&format!("refs/heads/{}", branch_name))
                .map_err(|e| {
                    crate::logger::error("GIT_BRANCHING", &format!("Failed to set HEAD to branch '{}'", branch_name), Some(&e.to_string()));
                    format!("Failed to set HEAD to branch '{}': {}", branch_name, e)
                })?;
            
            crate::logger::info("GIT_BRANCHING", &format!("Successfully created and checked out branch: {}", branch_name), None);
            
            // Drop git2 types before await (end of scope)
        }
    }
    
    // Now we're outside the git2 scope, safe to await
    // Save branch info to database (FIX: this was missing before)
    crate::logger::debug("GIT_BRANCHING", "Saving branch info to database", None);
    save_branch_info(db, game_id, &branch_name, None).await?;
    
    // Update active branch in database
    update_active_branch(db, game_id, &branch_name).await?;
    
    let result = if branch_exists {
        format!("Switched to existing save branch: {}", branch_name)
    } else {
        format!("Created save branch: {}", branch_name)
    };
    
    crate::logger::info("GIT_BRANCHING", &result, None);

    Ok(result)
}

/// Create a new branch (alias for create_save_checkpoint)
pub async fn create_save_branch(
    db: &std::sync::Arc<tokio::sync::Mutex<Database>>,
    master_repo_path: &str,
    game_id: i64, 
    branch_name: &str, 
    description: Option<&str>
) -> Result<(), String> {
    create_save_checkpoint(db, master_repo_path, game_id, branch_name).await?;
    save_branch_info(db, game_id, branch_name, description).await?;
    Ok(())
}

/// Switch to a branch
pub async fn switch_save_branch(
    db: &std::sync::Arc<tokio::sync::Mutex<Database>>,
    master_repo_path: &str,
    game_id: i64, 
    branch_name: &str
) -> Result<(), String> {
    crate::logger::info("GIT_BRANCHING", &format!("Switching to branch '{}' for game_id: {}", branch_name, game_id), None);
    
    {
        let repo = Repository::open(master_repo_path)
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", "Failed to open master repository", Some(&e.to_string()));
                format!("Failed to open master repository: {}", e)
            })?;

        // Find and checkout the branch
        let branch_ref = repo.find_branch(branch_name, git2::BranchType::Local)
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", &format!("Failed to find branch '{}'", branch_name), Some(&e.to_string()));
                format!("Failed to find branch '{}': {}", branch_name, e)
            })?;

        let branch_commit = branch_ref.get().peel_to_commit()
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", "Failed to get branch commit", Some(&e.to_string()));
                format!("Failed to get branch commit: {}", e)
            })?;

        repo.checkout_tree(&branch_commit.into_object(), None)
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", &format!("Failed to checkout branch '{}'", branch_name), Some(&e.to_string()));
                format!("Failed to checkout branch '{}': {}", branch_name, e)
            })?;

        repo.set_head(&format!("refs/heads/{}", branch_name))
            .map_err(|e| {
                crate::logger::error("GIT_BRANCHING", &format!("Failed to set HEAD to branch '{}'", branch_name), Some(&e.to_string()));
                format!("Failed to set HEAD to branch '{}': {}", branch_name, e)
            })?;
    }

    update_active_branch(db, game_id, branch_name).await?;
    
    crate::logger::info("GIT_BRANCHING", &format!("Successfully switched to branch: {}", branch_name), None);

    Ok(())
}

/// Get list of all branches
pub async fn list_all_branches(master_repo_path: &str) -> Result<Vec<String>, String> {
    crate::logger::debug("GIT_BRANCHING", "Listing all branches", None);
    
    let repo = Repository::open(master_repo_path)
        .map_err(|e| {
            crate::logger::error("GIT_BRANCHING", "Failed to open master repository", Some(&e.to_string()));
            format!("Failed to open master repository: {}", e)
        })?;

    let branches: Vec<String> = repo.branches(Some(git2::BranchType::Local))
        .map_err(|e| {
            crate::logger::error("GIT_BRANCHING", "Failed to get branches", Some(&e.to_string()));
            format!("Failed to get branches: {}", e)
        })?
        .filter_map(|b| b.ok())
        .map(|(b, _)| if let Ok(Some(s)) = b.name() { s.to_string() } else { "".to_string() })
        .collect();

    crate::logger::debug("GIT_BRANCHING", &format!("Found {} branches", branches.len()), None);

    Ok(branches)
}

/// Get branches for a specific game
pub async fn get_game_branches(master_repo_path: &str, game_name: &str) -> Result<Vec<String>, String> {
    crate::logger::debug("GIT_BRANCHING", &format!("Getting branches for game: {}", game_name), None);
    
    let all_branches = list_all_branches(master_repo_path).await?;
    
    // Filter branches that start with the game name followed by '-'
    let game_branches: Vec<String> = all_branches
        .into_iter()
        .filter(|branch| branch.starts_with(&format!("{}-", game_name)))
        .collect();

    crate::logger::debug("GIT_BRANCHING", &format!("Found {} branches for game '{}'", game_branches.len(), game_name), None);

    Ok(game_branches)
}

/// Delete a save branch
pub async fn delete_save_branch(master_repo_path: &str, branch_name: &str) -> Result<(), String> {
    crate::logger::info("GIT_BRANCHING", &format!("Deleting branch: {}", branch_name), None);
    
    let repo = Repository::open(master_repo_path)
        .map_err(|e| {
            crate::logger::error("GIT_BRANCHING", "Failed to open master repository", Some(&e.to_string()));
            format!("Failed to open master repository: {}", e)
        })?;

    // Find and delete the branch
    let mut branch = repo.find_branch(branch_name, git2::BranchType::Local)
        .map_err(|e| {
            crate::logger::error("GIT_BRANCHING", &format!("Failed to find branch '{}'", branch_name), Some(&e.to_string()));
            format!("Failed to find branch '{}': {}", branch_name, e)
        })?;

    branch.delete()
        .map_err(|e| {
            crate::logger::error("GIT_BRANCHING", &format!("Failed to delete branch '{}'", branch_name), Some(&e.to_string()));
            format!("Failed to delete branch '{}': {}", branch_name, e)
        })?;

    crate::logger::info("GIT_BRANCHING", &format!("Successfully deleted branch: {}", branch_name), None);

    Ok(())
}

async fn save_branch_info(
    db: &std::sync::Arc<tokio::sync::Mutex<Database>>,
    game_id: i64, 
    branch_name: &str, 
    description: Option<&str>
) -> Result<(), String> {
    let db_guard = db.lock().await;
    let conn = db_guard.get_connection().await;
    
    conn.execute(
        "INSERT INTO git_branches (game_id, branch_name, description, created_at)
            VALUES (?, ?, ?, ?)",
        rusqlite::params![
            game_id,
            branch_name,
            description.unwrap_or(""),
            Utc::now().to_rfc3339()
        ]
    ).map_err(|e| format!("Failed to save branch info: {}", e))?;
    
    Ok(())
}

async fn update_active_branch(
    db: &std::sync::Arc<tokio::sync::Mutex<Database>>,
    game_id: i64, 
    branch_name: &str
) -> Result<(), String> {
    let db_guard = db.lock().await;
    let conn = db_guard.get_connection().await;
    
    // Reset all branches for this game
    conn.execute(
        "UPDATE git_branches SET is_active = 0 WHERE game_id = ?",
        [game_id]
    ).map_err(|e| format!("Failed to reset active branches: {}", e))?;
    
    // Set current branch as active
    conn.execute(
        "UPDATE git_branches SET is_active = 1 WHERE game_id = ? AND branch_name = ?",
        rusqlite::params![game_id, branch_name]
    ).map_err(|e| format!("Failed to set active branch: {}", e))?;
    
    Ok(())
}
