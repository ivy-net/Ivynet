use std::{error::Error, fmt, fmt::Debug};

use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug)]
enum HexError {
    InvalidCharacter(char),
    InvalidStringLength(usize),
}

impl Error for HexError {}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidCharacter(char) => {
                write!(f, "Invalid character {char}")
            }
            Self::InvalidStringLength(length) => write!(f, "Invalid string length {length}"),
        }
    }
}

fn hex_decode<T: AsRef<[u8]>>(hex: T) -> Result<Vec<u8>, HexError> {
    let mut hex = hex.as_ref();
    let mut length = hex.len();

    if length == 42 && hex[0] == b'0' && (hex[1] == b'x' || hex[1] == b'X') {
        length -= 2;
        hex = &hex[2..];
    }
    if length != 40 {
        return Err(HexError::InvalidStringLength(length));
    }

    let hex_value = |char: u8| -> Result<u8, HexError> {
        match char {
            b'A'..=b'F' => Ok(char - b'A' + 10),
            b'a'..=b'f' => Ok(char - b'a' + 10),
            b'0'..=b'9' => Ok(char - b'0'),
            _ => Err(HexError::InvalidCharacter(char as char)),
        }
    };

    let mut bytes = Vec::with_capacity(length / 2);
    for chunk in hex.chunks(2) {
        let msd = hex_value(chunk[0])?;
        let lsd = hex_value(chunk[1])?;
        bytes.push(msd << 4 | lsd);
    }

    Ok(bytes)
}

#[proc_macro]
pub fn h160(input: TokenStream) -> TokenStream {
    let bytes = hex_decode(input.to_string()).expect("hex string");
    let expanded = quote! {
        H160([#(#bytes,)*])
    };

    expanded.into()
}
