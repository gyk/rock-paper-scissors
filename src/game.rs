use std::cmp::Ordering;
use std::str::FromStr;

use rand::{thread_rng, Rng};
use sha2::{Sha256, Digest};

use util::{bytes_to_hex, gen_random_bytes};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Hand {
    Rock,
    Paper,
    Scissors,
}

impl Hand {
    pub fn vs(&self, rhs: &Hand) -> Ordering {
        use self::Hand::*;

        if self == rhs {
            return Ordering::Equal;
        }
        match (*self, *rhs) {
            (Rock, Paper) => Ordering::Less,
            (Paper, Scissors) => Ordering::Less,
            (Scissors, Rock) => Ordering::Less,

            _ => Ordering::Greater,
        }
    }

    pub fn as_icon(&self) -> &'static str {
        match *self {
            Hand::Rock => "✊🏼",
            Hand::Paper => "✋🏼",
            Hand::Scissors => "✌🏼",
        }
    }

    const CHOICES: [Hand; 3] = [Hand::Rock, Hand::Paper, Hand::Scissors];

    pub fn random() -> Hand {
        let mut rng = thread_rng();
        *rng.choose(&Self::CHOICES).unwrap()
    }
}

impl AsRef<str> for Hand {
    fn as_ref(&self) -> &str {
        match *self {
            Hand::Rock => "rock",
            Hand::Paper => "paper",
            Hand::Scissors => "scissors",
        }
    }
}

pub struct ParseHandError;

impl FromStr for Hand {
    type Err = ParseHandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_ref() {
            "rock" => Hand::Rock,
            "paper" => Hand::Paper,
            "scissors" => Hand::Scissors,
            _ => return Err(ParseHandError),
        })
    }
}

pub struct Round {
    pub computer: Hand,
    pub random_bytes: String,
    pub digest: String,
}

impl Round {
    pub fn random() -> Round {
        let hand = Hand::random();
        let random_bytes = gen_random_bytes(32);
        let random_bytes_hex = bytes_to_hex(&random_bytes[..]);
        let concat_str = format!("{}{}", random_bytes_hex, hand.as_ref());

        let digest = format!("{:x}", Sha256::digest(concat_str.as_bytes()));
        Round {
            computer: hand,
            random_bytes: random_bytes_hex,
            digest: digest,
        }
    }
}
