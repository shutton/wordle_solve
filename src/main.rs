use anyhow::{anyhow, Result};
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fmt::Write;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    /// Enable words that are accepted, but can't be an answer. This more-accurately represents what the game allows.
    #[structopt(long)]
    more_words: bool,
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

    let mut omit_letters = vec![];
    let mut req_letters = vec![];
    let mut cand_letters = vec![];
    let mut rl = rustyline::Editor::<()>::new();
    let words: Vec<&str> = if opt.more_words {
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
        words = suggest(
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

fn suggest(hint: Hint, words: &[Word]) -> Vec<Word> {
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
    let mut scores: BTreeMap<i32, Vec<&Word>> = BTreeMap::new();
    words.iter().for_each(|word| {
        scores
            .entry(word.0.iter().unique().map(|c| -freq.get(&c).unwrap()).sum())
            .or_insert_with(Vec::new)
            .push(word)
    });

    // Display the top suggestions
    println!("Suggestions, in ascending order of score:");
    for (score, words) in scores.iter().take(10).rev() {
        println!("{:5} -> {:?}", -score, words);
    }

    words
}
