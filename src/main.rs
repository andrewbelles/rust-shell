use std::env;
use std::path::*;
use std::io::*; 
use std::process::*; 

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
    let mut commands = input.trim().split("|").peekable(); 
    let mut previous_command = None;    // tracks last ran
    
    while let Some(command) = commands.next() { 

        let mut parts = command.trim().split_whitespace(); 
        let command = parts.next().unwrap(); 
        let args = parts; 

        match command {
            // Built-In commands 
            "cd" => {
                let mut args = args.peekable();
                let new_dir = args.peek().copied().unwrap_or("/"); 
                let root = Path::new(new_dir);
                if let Err(e) = env::set_current_dir(&root) {
                    eprintln!("{}", e);
                }

                previous_command = None; 
            },
            "exit" => return false, 
            
            // Others
            command => {
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
                    .args(args)
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
        final_command.wait(); 
    }
    
    true 
}

fn main() {  

    // Shell loop 
    loop {
        print!("> ");
        stdout().flush().ok(); 

        let mut input = String::new(); 
        stdin().read_line(&mut input).unwrap(); 

        // Iterable over commands split by a pipeline 
        if !shell_run(input) { 
            return 
        }    
    }
}
