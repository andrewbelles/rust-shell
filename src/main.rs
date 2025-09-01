use std::env;
use std::path::*;
use std::io::*; 
use std::process::*; 

use glob::glob; 
use shellexpand; 
use whoami; 

/// 
/// Builds prompt to terminal 
///
/// Reads username, hostname, and gets relative cwd to home 
/// and formats a pretty display to be printed at the start of 
/// shell prompt 
///
fn prompt() -> String {
    // Get username and hostname 
    let user = whoami::username(); 
    let host = whoami::fallible::hostname()
        .unwrap_or_else(|_| "unknown".to_string());

    // Get current working direction -> String 
    let cwd  = env::current_dir().unwrap_or_else(|_| PathBuf::from("?"));
    let mut cwd_fmt = cwd.to_string_lossy().into_owned();

    // Remove home path from current path 
    if let Ok(home) = env::var("HOME") {
        if cwd_fmt.starts_with(&home) {
            cwd_fmt = cwd_fmt.replacen(&home, "~", 1);
        }
    }
    format!("{user}@{host}:{cwd_fmt}$ ")
}

/// 
/// Expands patterns in args to be used in command 
///
/// Input: 
///   Args with patterns still reduced. We denote that args implements iterator 
///   to allow us to iterate over and expand argument by argument 
///
/// Output: 
///   Vector of Strings where a single element is a single argument to consider 
///
///
fn expand_args<'a>(args: impl Iterator<Item=&'a str>) -> Vec<String> {
    let mut args_out = Vec::new(); 

    for arg in args {
        let expanded = shellexpand::full(arg)
            .unwrap_or_else(|_| arg.into())
            .into_owned();

        if expanded.contains(['*', '?', '[']) {
            match glob(&expanded) {
                Ok(paths) => {

                    let mut matched = false; 
                    for path in paths.flatten() {
                        matched = true; 
                        args_out.push(path.to_string_lossy().into_owned());
                    }
                    if !matched {
                        args_out.push(expanded);
                    }
                }
                Err(_) => args_out.push(expanded),
            }
        } else {
            args_out.push(expanded);
        }
    }
    
    args_out 
}

///
/// Finds first matching directory to input pattern 
///
/// Inputs: 
///   Optional string from current command args in shell_run  
///   We allow an option since cd None is a valid command (takes to home)
///
/// Output: 
///   Returns the full path to the new directory 
///
fn resolve_cd(dir: Option<&str>) -> String {
    let raw = dir.unwrap_or("~");
    let expanded = shellexpand::full(raw)
        .unwrap_or_else(|_| raw.into())
        .into_owned();

    if expanded.contains(['*', '?', '[']) {
        if let Ok(paths) = glob(&expanded) {
            for path in paths.flatten() {
                if path.is_dir() {
                    return path.to_string_lossy().into_owned();
                }
            }
        }
    }

    expanded
}

/// 
/// Main handler to run shell commands 
///
/// Inputs: 
///   string slice of command to run 
///   iterable string slice with lifetime through function 
///
/// Returns: 
///   false for failure to run command, that is, exit was specified
///   true else 
///
fn shell_run(input: String) -> bool {
    let mut commands = input.split("|")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .peekable(); 

    let mut previous_command: Option<std::process::Child> = None;
    
    while let Some(command) = commands.next() { 

        let mut parts = command.split_whitespace(); 
        let Some(command) = parts.next() else {
            continue; 
        }; 

        match command {
            // Built-In commands 
            "cd" => {
                let target_dir = resolve_cd(parts.next());
                let root = Path::new(&target_dir);
                if let Err(e) = env::set_current_dir(&root) {
                    eprintln!("{}", e);
                }

                previous_command = None; 
            },
            "exit" => return false, 
            
            // Others
            command => {
                let argv = expand_args(parts);
                let stdin = previous_command 
                    .map_or( 
                        Stdio::inherit(),
                        |output: Child| Stdio::from(output.stdout.unwrap())
                    );

                let stdout = if commands.peek().is_some() {
                    Stdio::piped()
                } else { 
                    Stdio::inherit()
                };

                let output = Command::new(command)
                    .args(&argv)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn(); 
                
                // If command is an error, handle 
                match output { 
                    Ok(output) => { previous_command = Some(output) },
                    Err(e) => {
                        previous_command = None; 
                        eprintln!("{}", e);
                    }
                };
            }
        }
    } 
    
    if let Some(mut final_command) = previous_command {
        let _ = final_command.wait();   // ignore Option  
    }
    
    true 
}

fn main() {  

    // Shell loop 
    loop {
        print!("{}", prompt());
        stdout().flush().ok(); 

        let mut input = String::new(); 
        stdin().read_line(&mut input).unwrap(); 

        // Iterable over commands split by a pipeline 
        if !shell_run(input) { 
            return 
        }    
    }
}
