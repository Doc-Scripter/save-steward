use git2::Repository;
use chrono::{DateTime, Utc};
use super::GitSaveCommit;
use super::GitSaveHistory;

/// Restore to a specific commit
pub async fn restore_to_commit(master_repo_path: &str, _game_id: i64, commit_hash: &str) -> Result<(), String> {
    let repo = Repository::open(master_repo_path)
        .map_err(|e| format!("Failed to open master repository: {}", e))?;

    // Find commit
    let commit = repo.find_commit(git2::Oid::from_str(commit_hash).map_err(|e| format!("Invalid commit hash: {}", e))?)
        .map_err(|e| format!("Failed to find commit '{}': {}", commit_hash, e))?;

    // Create new branch for this commit (optional)
    let timestamp = commit.time();
    let branch_name = format!("restore-{}-{}",
        chrono::DateTime::from_timestamp(timestamp.seconds(), 0).unwrap().format("%Y%m%d-%H%M%S"),
        commit_hash.chars().take(8).collect::<String>()
    );

    repo.branch(&branch_name, &commit, false)
        .map_err(|e| format!("Failed to create restore branch: {}", e))?;

    // Checkout the commit
    repo.checkout_tree(&commit.into_object(), None)
        .map_err(|e| format!("Failed to checkout commit '{}': {}", commit_hash, e))?;

    repo.set_head(&format!("refs/heads/{}", branch_name))
        .map_err(|e| format!("Failed to set HEAD to restore branch: {}", e))?;

    Ok(())
}

/// Restore to a timestamp (finds nearest commit)
pub async fn restore_to_timestamp(master_repo_path: &str, game_id: i64, target_time: DateTime<Utc>) -> Result<String, String> {
    let (commit_hash, commit_msg) = {
        let repo = Repository::open(master_repo_path)
            .map_err(|e| format!("Failed to open master repository: {}", e))?;

        let mut revwalk = repo.revwalk()
            .map_err(|e| format!("Failed to create revision walker: {}", e))?;

        revwalk.push_head()
            .map_err(|e| format!("Failed to push HEAD: {}", e))?;

        let mut closest_commit = None;
        let mut closest_time_diff = i64::MAX;

        for oid in revwalk {
            let oid = oid.map_err(|e| format!("Failed to get revision: {}", e))?;
            let commit = repo.find_commit(oid)
                .map_err(|e| format!("Failed to find commit: {}", e))?;

            let commit_time = commit.time();
            let commit_datetime = DateTime::from_timestamp(commit_time.seconds(), 0)
                .ok_or_else(|| "Invalid timestamp".to_string())?;

            let time_diff = (target_time.timestamp() - commit_datetime.timestamp()).abs();
            
            if time_diff < closest_time_diff {
                closest_time_diff = time_diff;
                closest_commit = Some(commit);
            }
        }

        if let Some(commit) = closest_commit {
            let hash = commit.id().to_string();
            let msg = commit.message().unwrap_or("Restore commit").to_string();
            (hash, msg)
        } else {
            return Err("No commits found".to_string());
        }
    };

    restore_to_commit(master_repo_path, game_id, &commit_hash).await?;
    
    Ok(format!("Restored to nearest commit: {} ({})", commit_hash.chars().take(8).collect::<String>(), commit_msg))
}

/// Get save history
pub async fn get_save_history(master_repo_path: &str, _game_id: i64) -> Result<serde_json::Value, String> {
    let repo = Repository::open(master_repo_path)
        .map_err(|e| format!("Failed to open master repository: {}", e))?;

    // Get current branch
    let current_branch = repo.head()
        .ok()
        .and_then(|r| r.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "detached".to_string());

    // Get all branches
    let branches: Vec<String> = repo.branches(Some(git2::BranchType::Local))
        .map_err(|e| format!("Failed to get branches: {}", e))?
        .filter_map(|b| b.ok())
        .map(|(b, _)| b.name().unwrap_or(None).map(|s| s.to_string()).unwrap_or_else(|| "".to_string()))
        .collect();

    // Get commit history
    let mut revwalk = repo.revwalk()
        .map_err(|e| format!("Failed to create revision walker: {}", e))?;

    revwalk.push_head()
        .map_err(|e| format!("Failed to push HEAD: {}", e))?;

    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid.map_err(|e| format!("Failed to get revision: {}", e))?;
        let commit = repo.find_commit(oid)
            .map_err(|e| format!("Failed to find commit: {}", e))?;

        let commit_time = commit.time();
        let commit_datetime = DateTime::from_timestamp(commit_time.seconds(), 0)
            .unwrap_or_else(|| Utc::now());

        // Extract game name from branch name
        let branch_name = current_branch.clone(); // This should be extracted from commit branch
        let game_name = if branch_name.contains('+') {
            branch_name.split('+').next().unwrap_or("Unknown").to_string()
        } else {
            "Unknown".to_string()
        };

        commits.push(GitSaveCommit {
            hash: commit.id().to_string(),
            message: commit.message().unwrap_or("No message").to_string(),
            timestamp: commit_datetime,
            branch: branch_name,
            game_name,
        });
    }

    let history = GitSaveHistory {
        commits,
        branches,
        current_branch,
    };

    serde_json::to_value(history).map_err(|e| format!("Failed to serialize history: {}", e))
}
