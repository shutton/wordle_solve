use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

// Words that are candidates for answers
const WORDS_USED_FILENAME: &str = "words-used.txt";
const WORDS_XTRA_FILENAME: &str = "words-extra.txt";

pub fn main() {
    // Parse the word lists and build into the binary.  They don't change, and there aren't that many
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("ENV{CARGO_MANIFEST_DIR}"));

    let mut wordfile =
        File::create(PathBuf::from(env::var("OUT_DIR").expect("ENV{OUT_DIR}")).join("words.rs"))
            .expect("File::create");

    let words_used: Vec<String> =
        BufReader::new(File::open(manifest_dir.join(WORDS_USED_FILENAME)).expect("open"))
            .lines()
            .take_while(Result::is_ok)
            .map(Result::unwrap)
            .collect();
    write!(
        wordfile,
        "pub const WORDS_USED: &[&str] = &{:?};",
        words_used
    )
    .expect("write");

    let words_used: Vec<String> =
        BufReader::new(File::open(manifest_dir.join(WORDS_XTRA_FILENAME)).expect("open"))
            .lines()
            .take_while(Result::is_ok)
            .map(Result::unwrap)
            .collect();
    write!(
        wordfile,
        "pub const WORDS_XTRA: &[&str] = &{:?};",
        words_used
    )
    .expect("write");
}
