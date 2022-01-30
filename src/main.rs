use anyhow::{anyhow, Result};
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fmt::Write;
use structopt::StructOpt;
use words::WORDS_USED;

const MAX_GUESSES: usize = 6;

#[derive(StructOpt)]
enum Opt {
    Solve {
        /// Enable words that are accepted, but can't be an answer. This more-accurately represents what the game allows.
        #[structopt(long)]
        more_words: bool,
    },

    Play,
}

mod words {
    include!(concat!(env!("OUT_DIR"), "/words.rs"));
}

#[derive(Debug)]
struct Hint<'a> {
    omit_letters: &'a [char],
    req_letters: &'a [char],
    cand_letters: Option<&'a [FoundLetter]>,
}

#[derive(Debug)]
struct FoundLetter {
    letter: char,
    position: usize,
    correct_location: bool,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
struct Word([char; 5]);

impl std::fmt::Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().for_each(|c| {
            f.write_char(*c).unwrap();
        });
        Ok(())
    }
}

impl std::fmt::Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().for_each(|c| {
            f.write_char(*c).unwrap();
        });
        Ok(())
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    match opt {
        Opt::Solve { more_words } => solve(more_words)?,
        Opt::Play => {
            let answer = random_answer();
            match play(answer) {
                Ok(guesses) => println!("Good job! It took you {} guesses", guesses),
                Err(_) => println!("Better luck next time.  The answer was \"{}\".", answer),
            }
        }
    }

    Ok(())
}

fn play(answer: Word) -> Result<usize> {
    let mut results = vec![];

    let mut rl = rustyline::Editor::<()>::new();
    for guess_no in 1..=MAX_GUESSES {
        let guess = 'try_guess: loop {
            let guess = rl.readline(format!("Guess {} of {}: ", guess_no, MAX_GUESSES).as_ref())?;
            match Word::try_from(guess.as_str()) {
                Err(_) => {
                    println!("Invalid guess.");
                    continue 'try_guess;
                }
                Ok(guess) => break guess,
            }
        };
        if guess == answer {
            println!("Correct!  It was \"{}\"", answer);
            return Ok(guess_no);
        } else {
            let gr = guess_word(guess, answer);
            results.push(gr);
            for (i, result) in results.iter().enumerate() {
                println!("{}. {}", i, result);
            }
        }
    }

    Err(anyhow!("Ran out of guesses"))
}

fn random_answer() -> Word {
    WORDS_USED
        .get(rand::random::<usize>() % WORDS_USED.len())
        .copied()
        .unwrap()
        .try_into()
        .unwrap()
}

fn solve(more_words: bool) -> Result<()> {
    let mut omit_letters = vec![];
    let mut req_letters = vec![];
    let mut cand_letters = vec![];
    let mut rl = rustyline::Editor::<()>::new();
    let words: Vec<&str> = if more_words {
        words::WORDS_USED
            .iter()
            .chain(words::WORDS_XTRA.iter())
            .copied()
            .collect()
    } else {
        words::WORDS_USED.iter().copied().collect()
    };

    let mut words: Vec<Word> = words
        .iter()
        .map(|&s| Word::try_from(s))
        .map(Result::unwrap)
        .collect();
    loop {
        let (new_words, scores) = suggest(
            Hint {
                omit_letters: &omit_letters,
                req_letters: &req_letters,
                cand_letters: if cand_letters.is_empty() {
                    None
                } else {
                    Some(&cand_letters)
                },
            },
            &words,
        );
        display_suggestions(&scores);
        words = new_words;
        'input: loop {
            match rl.readline("Result: ") {
                Ok(line) => {
                    let mut position = 0;
                    let mut negate_next = false;
                    for c in line.chars() {
                        match c {
                            '!' | '`' | '\'' => negate_next = true,
                            'a'..='z' => {
                                if negate_next {
                                    omit_letters.push(c);
                                    negate_next = false;
                                } else {
                                    req_letters.push(c);
                                    cand_letters.push(FoundLetter {
                                        letter: c,
                                        position,
                                        correct_location: false,
                                    })
                                }
                                position += 1;
                            }
                            'A'..='Z' => {
                                req_letters.push(c.to_ascii_lowercase());
                                cand_letters.push(FoundLetter {
                                    letter: c.to_ascii_lowercase(),
                                    position,
                                    correct_location: true,
                                });
                                position += 1;
                            }
                            _ => {
                                eprintln!("Invalid entry");
                                continue 'input;
                            }
                        }
                    }
                    break;
                }
                Err(e) => {
                    return Err(anyhow!("Error: {}", e));
                }
            }
        }
    }
}

fn is_candidate(word: &Word, hint: &Hint) -> bool {
    if !hint.omit_letters.is_empty() && word.0.iter().any(|c| hint.omit_letters.contains(c)) {
        return false;
    }
    if !hint.req_letters.is_empty() && !hint.req_letters.iter().all(|c| word.0.contains(c)) {
        return false;
    }
    // Now check all the positions
    if let Some(cands) = hint.cand_letters {
        for cand in cands {
            if cand.correct_location {
                if word.0[cand.position] != cand.letter {
                    return false;
                }
            } else if word.0[cand.position] == cand.letter {
                return false;
            }
        }
    }
    true
}

impl TryFrom<&str> for Word {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 5 {
            return Err(());
        }

        let mut word: Word = Word::default();
        value
            .chars()
            .enumerate()
            .take(5)
            .for_each(|(pos, c)| word.0[pos] = c.to_ascii_lowercase());

        Ok(word)
    }
}

fn suggest(hint: Hint, words: &[Word]) -> (Vec<Word>, BTreeMap<i32, Vec<Word>>) {
    let mut freq = BTreeMap::new();

    // Find the subset of possible matches based on the available hints
    let words: Vec<Word> = words
        .iter()
        .filter(|&word| is_candidate(word, &hint))
        .copied()
        .collect();

    // Determine the frequencies
    words.iter().for_each(|word| {
        word.0
            .iter()
            .for_each(|letter| *(freq.entry(letter).or_insert(0)) += 1)
    });

    // We really want this map to be ordered by highest score, but that requires
    // implementing a wrapper type around numbers. It's easier to just negate the
    // score so the map is ordered as desired.
    let mut scores: BTreeMap<i32, Vec<Word>> = BTreeMap::new();
    words.iter().for_each(|&word| {
        scores
            .entry(word.0.iter().unique().map(|c| -freq.get(&c).unwrap()).sum())
            .or_insert_with(Vec::new)
            .push(word)
    });

    (words, scores)
}

fn display_suggestions(scores: &BTreeMap<i32, Vec<Word>>) {
    // Display the top suggestions
    println!("Suggestions, in ascending order of score:");
    for (score, words) in scores.iter().take(10).rev() {
        println!("{:5} -> {:?}", -score, words);
    }
}

struct GuessResult([GuessLetter; 5]);

impl std::fmt::Display for GuessResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for gc in self.0 {
            write!(f, "{}", gc)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum GuessLetter {
    Empty,
    Correct(char),
    Present(char),
    Incorrect(char),
}

impl std::fmt::Display for GuessLetter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use owo_colors::AnsiColors;
        use owo_colors::DynColor;
        match self {
            GuessLetter::Empty => f.write_char(' '),
            GuessLetter::Correct(c) => {
                AnsiColors::Green.fmt_ansi_bg(f)?;
                AnsiColors::Black.fmt_ansi_fg(f)?;
                f.write_char(c.to_ascii_uppercase())
            }
            GuessLetter::Present(c) => {
                AnsiColors::Yellow.fmt_ansi_bg(f)?;
                AnsiColors::Black.fmt_ansi_fg(f)?;
                f.write_char(c.to_ascii_uppercase())
            }
            GuessLetter::Incorrect(c) => {
                AnsiColors::BrightBlack.fmt_ansi_bg(f)?;
                AnsiColors::BrightWhite.fmt_ansi_fg(f)?;
                f.write_char(c.to_ascii_uppercase())
            }
        }?;
        AnsiColors::Black.fmt_ansi_bg(f)?;
        AnsiColors::White.fmt_ansi_fg(f)
    }
}

fn guess_word(guess: Word, answer: Word) -> GuessResult {
    let mut result = [GuessLetter::Empty; 5];

    for (pos, (&guess_char, &answer_char)) in guess.0.iter().zip(answer.0.iter()).enumerate() {
        result[pos] = if guess_char == answer_char {
            GuessLetter::Correct(guess_char)
        } else if answer.0.iter().any(|&c| c == guess_char) {
            GuessLetter::Present(guess_char)
        } else {
            GuessLetter::Incorrect(guess_char)
        }
    }

    GuessResult(result)
}

#[test]
fn test_guess_word() {
    let guess = "abcde".try_into().unwrap();
    let answer = "bacfe".try_into().unwrap();
    eprintln!("{}", guess_word(guess, answer));
}
