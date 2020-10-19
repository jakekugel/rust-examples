// Interactive command-line program to encode or decode a message to or from a deck of playing cards.
//
// Author: Jake Kugel

use std::io::{self, Write};
use num_bigint::BigInt;
use num_bigint::ToBigInt;

use num_traits::{Zero, One};
use num_traits::cast::ToPrimitive;

use std::process;

static ALPHABET: [char; 32] = [
    'a',
    'b',
    'c',
    'd',
    'e',
    'f',
    'g',
    'h',
    'i',
    'j',
    'k',
    'l',
    'm',
    'n',
    'o',
    'p',
    'q',
    'r',
    's',
    't',
    'u',
    'v',
    'w',
    'x',
    'y',
    'z',
    ' ',
    '.',
    '!',
    '?',
    '-',
    '*'
];

const CARDS: &'static [&'static str] = &[
    "AC",
    "2C",
    "3C",
    "4C",
    "5C",
    "6C",
    "7C",
    "8C",
    "9C",
    "TC",
    "JC",
    "QC",
    "KC",
    "AD",
    "2D",
    "3D",
    "4D",
    "5D",
    "6D",
    "7D",
    "8D",
    "9D",
    "TD",
    "JD",
    "QD",
    "KD",
    "AH",
    "2H",
    "3H",
    "4H",
    "5H",
    "6H",
    "7H",
    "8H",
    "9H",
    "TH",
    "JH",
    "QH",
    "KH",
    "AS",
    "2S",
    "3S",
    "4S",
    "5S",
    "6S",
    "7S",
    "8S",
    "9S",
    "TS",
    "JS",
    "QS",
    "KS"
];

fn main() {
    print!("Would you like to encode or decode a message in a deck of cards?  Type 'encode' or 'decode' [encode]: ");
    io::stdout().flush().unwrap();

    let mut operation = String::new();

    io::stdin()
        .read_line(&mut operation)
        .expect("Failed to read line");

    operation = operation.trim().to_string().to_lowercase();

    if operation == "encode" || operation == "" {
        print!("Enter your message, 45 characters maximum.  Use a-z, ' ', '.', '!', '?', '-', and '*' : ");
        io::stdout().flush().unwrap();

        let mut message = String::new();

        io::stdin()
            .read_line(&mut message)
            .expect("Failed to read line");
        let message = message.trim().to_string().to_lowercase();

        // Convert message string to string representation of cards in a deck
        let cards: String = encode(message);

        println!("{}", cards);
    } else if operation == "decode" {
        print!("Enter list of 52 cards, separated by spaces: ");
        io::stdout().flush().unwrap();

        let mut cards = String::new();

        io::stdin()
            .read_line(&mut cards)
            .expect("Failed to read line");
        let cards = cards.trim().to_string();

        // Convert message string to string representation of cards in a deck
        let message: String = decode(cards);

        println!("{}", message);
    } else {
        println!("Error: must enter 'encode', 'decode'.");
        process::exit(1);
    }
}

/// Given a message, return a string representation of a deck of playing cards
fn encode(message: String) -> String {

    if message.len() > 45 {
        println!("Error: length of message may not be greater than 45 characters.");
        process::exit(1);
    }



    let number: BigInt = message_to_number(message);
    let cards: String = number_to_cards(&number);
    return cards;
}

/// Given a list of cards, return a decoded message.
fn decode(cards: String) -> String {
    let number: BigInt = cards_to_number(cards);
    let message: String = number_to_message(number);

    return message;
}

/// Given a numeric representation of a message, convert to text
fn number_to_message(number: BigInt) -> String {
    let mut message: String = "".to_string();

    // Copy number to a local variable
    let number_str: String = number.to_str_radix(10);
    let mut number_local: BigInt = number_str.parse::<BigInt>().unwrap();

    // Iterate through characters in the message, adding 5 binary digits to the
    // result number for each character.
    while number_local > Zero::zero() {
        message.push(num_to_char((&number_local % 32 as u32).to_u32().unwrap()));
        number_local = &number_local / 32;
    }

    return message;
}

/// Given a list of cards, convert to a number representation
fn cards_to_number(cards_string: String) -> BigInt {
    let mut cards: Vec<&str> = cards_string.split(" ").collect();
    let mut result: BigInt = Zero::zero();

    if cards.len() != 52 {
        println!("Error: expected 52 cards, received {}.", cards.len());
        process::exit(1)
    }

    // Iterate through all 52 cards in order.  For each card, check its
    // position in the encoded deck.  Multiply position by 51!, then
    // remove from the encoded deck.  Repeat with second card, and 50!.
    for (index, card) in CARDS.iter().enumerate() {
        let position: BigInt = remove_card(card, &mut cards);
        let subrange_size: BigInt = factorial(51 - index as u32);
        result = result + position * subrange_size;
    }

    return result;
}

/// Given a search card and a list (deck) of cards, find the position of the search
/// card in the cards, and remove the card from the list of cards.
fn remove_card(search_card: &str, cards: &mut Vec<&str>) -> BigInt {

    for (index, card) in cards.iter().enumerate() {
        if &search_card == card {
            cards.remove(index);
            return index.to_bigint().unwrap();
        }
    }

    println!("Error: the card '{}' was not found in the list of cards.", search_card);
    process::exit(1);
}

/// Converts given string to a large number.
fn message_to_number(message: String) -> BigInt {
    let mut result: BigInt = Zero::zero();
    let mut multiplier: BigInt = One::one();

    // Iterate through characters in the message, adding 5 binary digits to the
    // result number for each character.
    for c in message.chars() {
        result = result + lookup_char(c) * &multiplier;
        multiplier = multiplier * 32;
    }

    return result;
}

/// Converts large number to a string representation of playing cards
fn number_to_cards(number: &BigInt) -> String {
    let mut cards: Vec<String> = vec![String::new(); 52];

    // Copy number to a local variable
    let number_str: String = number.to_str_radix(10);
    let mut remainder: BigInt = number_str.parse::<BigInt>().unwrap();

    // Iterate through all 52 cards.  For each card, determine its
    // position in the deck by dividing the total range of the
    // number into 52 sub-ranges (each of these sub-ranges being
    // 51! in size).  Take the remainder and repeat the process, this
    // time dividing the space into 51 sub-ranges, each of size 50!.
    for (index, card) in CARDS.iter().enumerate() {
        // Compute 51! in first iteration, then 50!, then 49!, etc
        let subrange_size: BigInt = factorial(51 - index as u32);
        let position: BigInt = &remainder / &subrange_size;
        remainder = remainder - (&position * &subrange_size);

        // Hmm, is there an easier way to convert BigInt to u32?
        // Maybe this is easier? https://stackoverflow.com/a/50485697
        let position_str: String = position.to_str_radix(10);
        let position_u32: u32 = position_str.parse::<u32>().unwrap();

        // Search the cards array for the open position at 'position'.
        let mut count_open_slots: u32 = 0;
        for (index, card_in_results) in cards.iter().enumerate() {
            if card_in_results == "" {
                if position_u32 == count_open_slots {
                    cards[index] = (*card).to_string();
                    break;
                } else {
                    count_open_slots = count_open_slots + 1;
                }
            }
        }
    }

    // Create a string representation
    let mut result: String = String::new();
    for (index, card) in cards.iter().enumerate() {
        result = result + card;

        if index != cards.len() - 1 {
            result = result + " "
        }
    }

    return result;
}

/// Given a char, return a 5 bit number (0-31) representing char
fn lookup_char(char: char) -> u32 {
    for (index, alphabet_char) in ALPHABET.iter().enumerate() {
        if char == *alphabet_char {
            return index as u32;
        }
    }

    println!("Error: the character '{}' is not one of the valid characters: a-z, ' ', '.', '!', '?', '-', and '*'.", char);
    process::exit(1);
}


/// Number 0-31, return char representation from alphabet used for messages
fn num_to_char(number: u32) -> char {
    return *ALPHABET.get(number as usize).unwrap();
}

/// Compute a factorial giving result as a BigInt
fn factorial(operand: u32) -> BigInt {
    let mut result: BigInt = One::one();
    for multiplicand in 1..(operand + 1) {
        result = result * multiplicand;
    }
    return result;
}
