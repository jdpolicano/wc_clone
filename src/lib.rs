/*
Description
Print newline, word, and byte counts for each FILE
-c, --bytes
print the byte counts

-m, --chars
print the character counts

-l, --lines
print the newline counts

-w, --words
print the word counts

--help
display this help and exit

--version
output version information and exit

default is lines, chars, bytes....
*/
use std::env;
use std::error::Error;
use std::fs;
use std::io;

#[derive(Debug)] 
pub struct CommandOptions {
    count_words: bool,
    count_chars: bool,
    count_bytes: bool,
    count_lines: bool,
    files: Vec<String>,
}


impl CommandOptions{
    fn new() -> Self {
        Self {
            count_bytes: false,
            count_chars: false,
            count_words: false,
            count_lines: false,
            files: Vec::new()
        }
    }
}

#[derive(Debug)]
pub struct FileStats {
    word_count: i32,
    char_count: i32,
    byte_count: i32,
    line_count: i32,
}

impl FileStats {
    fn new() -> Self {
        Self {
            word_count: 0,
            char_count: 0,
            byte_count: 0,
            line_count: 0,
        }
    }

    fn add(&mut self, other: &FileStats) {
        self.word_count += other.word_count;
        self.char_count += other.char_count;
        self.byte_count += other.byte_count;
        self.line_count += other.line_count;
    }
}


pub enum FileReadResult {
    Utf8(String),
    Binary(Vec<u8>),
    ReadError(Box<dyn Error>)
}

impl CommandOptions {
    pub fn build(mut argv: impl Iterator<Item=String>) -> Result<CommandOptions, String> {
        argv.next(); // assume for now the exec path is the first arg and skip it...

        let mut built_commands = CommandOptions::new();

        let mut use_default_options = true;
        // should parse the command line arguments consuing the arguments and updating the "built_commands" until it reaches 
        // an argument that doesn't start with "-" or "--"... This needs to be reworked so it cna handle multiple flags in one i.e., "-clm"
        while let Some(s) = argv.next() {
            if s.starts_with("-") {
                use_default_options = false; // make sure to
                match s.as_str() {
                    "--bytes" => built_commands.count_bytes = true,
                    "--chars" => built_commands.count_chars = true,
                    "--words" => built_commands.count_words = true,
                    "--lines" => built_commands.count_lines = true,
                    _ => {
                        for c in s[1..].chars() {
                            match c {
                                'c' => built_commands.count_bytes = true,
                                'm' => built_commands.count_chars = true,
                                'w' => built_commands.count_words = true,
                                'l' => built_commands.count_lines = true,
                                _ => return Err(format!("Recieved unsupported option: {}", s))
                            };
                        }
                    }
            
                }
            } else {
                built_commands.files.push(s);
                built_commands.files.append(&mut argv.collect());
                break;
            }
        }

        if use_default_options {
            built_commands.count_lines = true;
            built_commands.count_words = true;
            built_commands.count_bytes = true;
        }

        if built_commands.files.len() < 1 {
            return Err(String::from("No files spcified..."));
        }

        Ok(built_commands)
    }
}


pub fn run() {
    match CommandOptions::build(env::args()) {
        Ok(mut command_options) => {
            let mut all_stats: Vec<(FileStats, &str)> = Vec::new();
            let mut aggregated_stats = FileStats::new();

            for file in &command_options.files {
                match read_file(&file) {
                    FileReadResult::Utf8(utf8) => { 
                        let file_stats = get_file_stats(&utf8);
                        all_stats.push((file_stats, file));
                    },
                    FileReadResult::Binary(bin) => { 
                        // this is very simple and probably incorrect but enough for now, this is a learning exercise :).
                        if command_options.count_chars {
                            println!("wc_clone: {} Illegal byte sequence", file); 
                            command_options.count_chars = false;
                        }
                        let file_stats = get_file_stats_bin(&bin);
                        all_stats.push((file_stats, file));
                    },
                    FileReadResult::ReadError(err) => {
                        println!("Encounted error reading file {}: {}", file, err)
                    }
                }
            }

            for (stats, topic) in &all_stats {
                aggregated_stats.add(&stats);
                print_run_results(&command_options, &stats, topic)
            }

            if all_stats.len() > 1 {
                print_run_results(&command_options, &aggregated_stats, "total")
            }
        },

        Err(err) => println!("{}", err)
    };
}

/*
Reads a file as utf8 and falls back to processing as byte vec if unable to parse as valid utf8..
*/
pub fn read_file(path: &str) -> FileReadResult {
    match fs::read_to_string(path) {
        Ok(utf8_file) => FileReadResult::Utf8(utf8_file),
        Err(io_err) => {
            if io_err.kind() == io::ErrorKind::InvalidData {
                match fs::read(path) {
                    Ok(binary_file) => FileReadResult::Binary(binary_file),
                    Err(err) => FileReadResult::ReadError(Box::new(err)),
                }
            } else {
                FileReadResult::ReadError(Box::new(io_err))
            }
        }
    }
}

/*
Same as utf8 implementation, only it operates on binary directly...
*/
fn get_file_stats(file_content: &str) -> FileStats {
    let mut run_results = FileStats::new();

    run_results.byte_count = file_content.len() as i32;
    let mut in_word = false; // Keep track if we're inside a word

    for c in file_content.chars() {
        run_results.char_count += 1;

        if c == '\n' {
            run_results.line_count += 1;
            if in_word {
                run_results.word_count += 1;
                in_word = false;
            }
        } else if c == ' ' || c == '\t' || c == '\r' {
            if in_word {
                run_results.word_count += 1;
                in_word = false;
            }
        } else {
            in_word = true;
        }
    }

    // Check if the last word continues to the end of the content
    if in_word {
        run_results.word_count += 1;
    }

    run_results
}

/*
Prints run results based on the user configuration and a utf8 string...will return a 4 len vec containing the count of each data point.
This is useful for aggregating the results...
*/
fn get_file_stats_bin(file_content: &[u8]) -> FileStats {
    let mut run_results = FileStats::new();

    run_results.byte_count = file_content.len() as i32;
    let mut in_word = false; // Keep track if we're inside a word

    for byte in file_content {
        run_results.char_count += 1;

        if *byte == b'\n' {
            run_results.line_count += 1;
            if in_word {
                run_results.word_count += 1;
                in_word = false;
            }
        } else if *byte == b' ' || *byte == b'\t' || *byte == b'\r' {
            if in_word {
                run_results.word_count += 1;
                in_word = false;
            }
        } else {
            in_word = true;
        }
    }

    // Check if the last word continues to the end of the content
    if in_word {
        run_results.word_count += 1;
    }

    run_results
}


/*
Prints results based on a vec of stats and a topic
*/
fn print_run_results(options: &CommandOptions, stats: &FileStats, topic: &str) {
    let mut results = String::new();

    if options.count_lines {
        results.push_str(format!(" {}", stats.line_count).as_str());
    }

    if options.count_words {
        results.push_str(format!(" {}", stats.word_count).as_str());
    }

    if options.count_chars {
        results.push_str(format!(" {}", stats.char_count).as_str());
    }

    if options.count_bytes {
        results.push_str(format!(" {}", stats.byte_count).as_str());
    }

    results.push_str(format!(" {}", topic).as_str());
    println!("{results}");
}




