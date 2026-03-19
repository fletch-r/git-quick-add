use dialoguer::Input;
use git2::Repository;
use regex::Regex;
use std::process::{Command, Stdio};
use std::str;

pub fn commit(repo: &Repository) {
    // 0: We have our staged changes
    // 1: Prompt the user for a commit message
    let commit_message: String = Input::new()
        .with_prompt("Enter Commit Message")
        .interact_text()
        .unwrap();
    // 2: Get branch id
    let branch_name = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(str::to_owned))
        .unwrap_or_else(|| "HEAD".to_string());
    let branch_segment = branch_name.rsplit('/').next().unwrap_or(&branch_name);
    let reference_id = Regex::new(r"^([^\d]*)(\d+)")
        .unwrap()
        .captures(branch_segment)
        .map(|captures| format!("{}{}", &captures[1], &captures[2]))
        .unwrap_or(branch_name);
    // 3: Create the commit
    // git commit -S -m "{reference_id}: {commit_message}"
    let full_commit_message = format!("{reference_id}: {commit_message}");
    let mut index = repo.index().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let signature = repo.signature().unwrap();
    let parent_commit = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repo.find_commit(oid).ok());
    let parents = parent_commit.iter().collect::<Vec<_>>();
    let commit_result = repo
        .commit_create_buffer(
            &signature,
            &signature,
            &full_commit_message,
            &tree,
            &parents,
        )
        .ok()
        .and_then(|commit_buffer| {
            let commit_content = str::from_utf8(&commit_buffer).ok()?;
            let signed_commit = sign_commit_buffer(repo, commit_content).ok()?;
            let commit_oid = repo.commit_signed(commit_content, &signed_commit, None).ok()?;
            update_head_to_commit(repo, commit_oid, &full_commit_message).ok()?;
            Some(commit_oid)
        });

    if commit_result.is_some() {
        println!(
            "{}",
            console::style("Signed commit created successfully").green()
        );
    } else {
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &full_commit_message,
            &tree,
            &parents,
        )
        .unwrap();
        println!(
            "{}",
            console::style("Commit created successfully (unsigned)").yellow()
        );
    }

    println!("Commit message: {full_commit_message}");
    // 5: Optional push
}

fn sign_commit_buffer(repo: &Repository, commit_content: &str) -> Result<String, git2::Error> {
    let config = repo.config()?;
    let signing_key = config.get_string("user.signingkey").ok();
    let gpg_program = config
        .get_string("gpg.program")
        .unwrap_or_else(|_| "gpg".to_string());

    let mut command = Command::new(gpg_program);
    command.arg("--armor").arg("--detach-sign");

    if let Some(signing_key) = signing_key.as_deref() {
        command.arg("--local-user").arg(signing_key);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| git2::Error::from_str(&format!("failed to start gpg: {err}")))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| git2::Error::from_str("failed to open gpg stdin"))?;
        use std::io::Write;
        stdin
            .write_all(commit_content.as_bytes())
            .map_err(|err| git2::Error::from_str(&format!("failed to write commit to gpg: {err}")))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| git2::Error::from_str(&format!("failed to wait for gpg: {err}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(git2::Error::from_str(&format!(
            "gpg failed to sign commit: {}",
            stderr.trim()
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|err| git2::Error::from_str(&format!("invalid gpg output: {err}")))
}

fn update_head_to_commit(
    repo: &Repository,
    commit_oid: git2::Oid,
    reflog_message: &str,
) -> Result<(), git2::Error> {
    let head = repo.head()?;

    match head.resolve() {
        Ok(mut direct_ref) => {
            direct_ref.set_target(commit_oid, reflog_message)?;
            Ok(())
        }
        Err(_) => repo.set_head_detached(commit_oid),
    }
}
