use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::Context;
use anyhow::Result;

fn main() -> Result<()> {
    let workspace = env::args().nth(1).expect("No workspace given");
    println!("Analyzing {}", workspace);

    let files = exec(&workspace, "git", "ls-tree -r --name-only HEAD -- .")?;

    let mut results = Vec::new();

    for file in files {
        println!("  {}", file);

        if contains_todos(&workspace, &file).map_or(false, |x| !x) {
            continue;
        }

        for (date, comment) in blame(&workspace, &file)? {
            results.push((file.clone(), date.to_string(), comment.to_string()));
        }
    }

    results.sort_by_key(|k| k.1.clone());

    for (file, date, comment) in results {
        println!("{}\t{}\n  {}", date, comment, file);
    }

    Ok(())
}

fn blame(workspace: &str, file: &str) -> Result<Vec<(String,String)>> {
    let args = format!("blame -c {}", file);
    let blame_output = exec(&workspace, "git", &args)?;

    let todo_lines = blame_output.iter()
        .filter(|line| line.to_uppercase().contains("TODO"))
        .map(|line| {
            let tokens: Vec<_> = line.split('\t').collect();
            let date = tokens[2];
            let comment_start = tokens[3].find(')').unwrap_or(0) + 1;
            let comment = &tokens[3][comment_start..].trim();
            (date.to_string(), comment.to_string())
        })
        .collect();

    Ok(todo_lines)
}

fn contains_todos(workspace: &str, file: &str) -> Result<bool> {
    let filepath = Path::new(workspace).join(file);
    let contents = fs::read_to_string(filepath.clone())
        .with_context(|| format!("Could not read file '{:?}'", filepath))?;

    let contains_todo = contents.lines()
        .filter(|line| line.to_uppercase().contains("TODO"))
        .next()
        .is_some();

    Ok(contains_todo)
}

fn exec(working_directory: &str, cmd: &str, args: &str) -> Result<Vec<String>> {
    let process = Command::new(cmd)
        .args(args.split_whitespace())
        .current_dir(working_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = process.stdout.with_context(|| {
        format!("Could not capture standard output of '{} {}'", cmd, args)
    })?;
    
    let reader = io::BufReader::new(output);

    Ok(reader.lines().filter_map(io::Result::ok).collect())
}
