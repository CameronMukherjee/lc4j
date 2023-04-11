// Nice feature, use Neo4J to visualise this?
// Add config.yml to configure scoring, output and more?

use std::collections::LinkedList;
use std::fs;
use std::fs::{File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use clap::{App, Arg};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
struct Directory {
    path: String,
    score: i32,
    text_lines: i32,
    files: LinkedList<DirectoryFile>,
    directories: LinkedList<Directory>
}

#[derive(Clone, Debug, Serialize)]
struct DirectoryFile {
    file_name: String,
    text_lines: i32,
    score: i32
}

fn main() {
    let matches = App::new("w8")
        .version("1.0")
        .author("Cameron Mukherjee <cameron@hexploits.com>")
        .about("Get file, package and directory weights for your project.")
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .value_name("PATH")
                .help("Sets the path to the target directory")
                .takes_value(true),
        )
        .get_matches();

    // Set the parent directory.
    let mut project_start_directory = std::env::current_dir()
        .expect("Failed to get working directory")
        .to_string_lossy()
        .to_string();

    if let Some(path) = matches.value_of("path") {
        project_start_directory = path.to_string();
    }

    let mut parent_directory = Directory{
        path: project_start_directory.to_string(),
        score: 0,
        text_lines: 0,
        directories: Default::default(),
        files: Default::default(),
    };

    let start_time = Instant::now();
    parent_directory = read_directory(parent_directory);

    // Set the parent directory scores.
    parent_directory.score = parent_directory.directories
        .iter()
        .map(|dir| dir.score)
        .fold(0, |sum, score| sum + score);
    parent_directory.text_lines = parent_directory.directories
        .iter()
        .map(|dir| dir.text_lines)
        .fold(0, |sum, score| sum + score);

    save_report(&parent_directory);
    let elapsed_time = Instant::now() - start_time;
    print_report(&parent_directory, elapsed_time);
}

fn print_report(directory: &Directory, elapsed_time: Duration) {
    println!("Path Processed: {}", directory.path);
    println!("Lines of Code: {}", directory.text_lines);
    println!("Code Complexity (w8-score): {}", directory.score);
    println!("Files Processed: {}", get_total_files_read(directory));
    println!("Time Taken: {:.2?}", elapsed_time)
}

fn get_total_files_read(directory: &Directory) -> i32 {
    let mut total_files = directory.files.len() as i32;

    for sub_directory in directory.directories.iter() {
        total_files += get_total_files_read(sub_directory);
    }

    total_files as i32
}

fn save_report(directory: &Directory) {
    let json = serde_json::to_string_pretty(&directory).unwrap();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let file_name = format!("./.w8-out/{}.json", timestamp.to_string());
    match fs::create_dir("./.w8-out") {
        _ => {} // TODO - only ignore "AlreadyExists" error.
    }
    let mut file = File::create(file_name).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}

fn read_directory(mut directory: Directory) -> Directory {
    let mut directory_files_list: LinkedList<DirectoryFile> = LinkedList::new();

    let paths = fs::read_dir(&directory.path).unwrap();
    for entry in paths {
        let path = entry.unwrap().path();
        let path_str = path.clone().into_os_string().into_string().unwrap();
        if path.is_dir() {
            let new_directory = handle_directory(&path_str);
            if new_directory.score != 0 {
                directory.directories.push_back(new_directory);
            }
        } else {
            let dir_file = path.extension()
                .map(|file_ext| file_ext.to_str())
                .flatten()
                .filter(|&file_ext| file_ext == "java")
                .map(|_| read_java_file(&path_str));

            if dir_file.is_some() {
                directory_files_list.push_back(dir_file.unwrap());
            }
        }
    }

    fn handle_directory(path_str: &str) -> Directory {
        let d = Directory{
            path: path_str.to_string(),
            score: 0,
            text_lines: 0,
            directories: Default::default(),
            files: Default::default(),
        };
        read_directory(d)
    }

    directory.files = directory_files_list.iter()
        .filter(|file| file.score != 0)
        .cloned()
        .collect();
    directory.score = calculate_directory_score(&directory);
    directory.text_lines = calculate_directory_text_lines(&directory);
    directory
}

// Recursive function to read through all directories and calculate the score.
fn calculate_directory_score(directory: &Directory) -> i32 {
    // Return the current directories score + the score for the children.
    let sub_directory_score: i32 = directory.directories
        .iter()
        .map(|sub_dir| calculate_directory_score(sub_dir))
        .sum();

    let this_files_score: i32 = directory.files
        .iter()
        .map(|file| file.score)
        .sum();

    return sub_directory_score + this_files_score;
}

fn calculate_directory_text_lines(directory: &Directory) -> i32 {
    let sub_directory_lines: i32 = directory.directories
        .iter()
        .map(|sub_dir| calculate_directory_text_lines(sub_dir))
        .sum();

    let this_text_lines: i32 = directory.files
        .iter()
        .map(|file| file.text_lines)
        .sum();

    return sub_directory_lines + this_text_lines;
}

fn read_java_file(path: &str) -> DirectoryFile {
    let mut file_complexity_score = 0;
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    let mut text_lines = 0;
    for line in reader.lines() {
        file_complexity_score += token_counter(&line.unwrap());
        text_lines += 1;
    }

    let file_name = Path::new(path)
        .file_name()
        .map(|os_str| os_str.to_string_lossy().into_owned())
        .unwrap_or("".to_string());

    DirectoryFile{ file_name, text_lines, score: file_complexity_score }
}

fn token_counter(line_of_code: &str) -> i32 {
    let mut complexity_score = 0;

    let line_of_code_trimmed = line_of_code.trim();
    if !line_of_code_trimmed.is_empty() || !(line_of_code_trimmed.len() == 1) {
        complexity_score += 1
    }

    // Does not account for comments.
    complexity_score += line_of_code.matches("if ").count() as i32;
    complexity_score += 3 * line_of_code.matches("for ").count() as i32;
    complexity_score += 3 * line_of_code.matches(".map").count() as i32;
    complexity_score += 3 * line_of_code.matches(".stream").count() as i32;
    complexity_score += 3 * line_of_code.matches(".flatMap").count() as i32;
    complexity_score += 3 * line_of_code.matches(".flatMapIterable").count() as i32;
    complexity_score += 3 * line_of_code.matches(".expand").count() as i32;

    complexity_score
}