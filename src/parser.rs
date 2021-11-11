use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::char,
    character::is_alphanumeric,
    combinator::map,
    error::ErrorKind,
    multi::{many0, many0_count},
    sequence::{preceded, terminated, tuple},
    IResult,
};

#[derive(Debug, PartialEq)]
enum ParseError<I> {
    InputEmpty,
    Nom(I, ErrorKind),
}

impl<I> nom::error::ParseError<I> for ParseError<I> {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        ParseError::Nom(input, kind)
    }

    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }
}

#[derive(Debug)]
pub enum Token<'a> {
    Text(&'a str),
    Variable { name: &'a str, raw: &'a str },
}

fn is_valid_variable_char(c: char) -> bool {
    let c = c as u8;
    is_alphanumeric(c)
        || c == b'.'
        || c == b'-'
        || c == b'_'
        || c == b'!'
        || c == b'@'
        || c == b'$'
        || c == b'#'
}

fn variable_name(i: &str) -> IResult<&str, &str, ParseError<&str>> {
    take_while(is_valid_variable_char)(i)
}

fn space_count(i: &str) -> IResult<&str, usize, ParseError<&str>> {
    many0_count(char(' '))(i)
}

fn parse_variable(i: &str) -> IResult<&str, Token, ParseError<&str>> {
    map(
        tuple((
            preceded(tag("{%"), space_count),
            variable_name,
            terminated(space_count, tag("%}")),
        )),
        |(count1, name, count2)| Token::Variable {
            name,
            raw: &i[..name.len() + 4 + count1 + count2],
        },
    )(i)
}

#[inline]
fn is_not_variable_start(chr: char) -> bool {
    chr != '{'
}

fn parse_text(i: &str) -> IResult<&str, Token, ParseError<&str>> {
    if i.is_empty() {
        return Err(nom::Err::Error(ParseError::InputEmpty));
    }

    map(take_while(is_not_variable_start), Token::Text)(i)
}

fn parse_brace(i: &str) -> IResult<&str, Token, ParseError<&str>> {
    map(tag("{"), Token::Text)(i)
}

fn parse_token(i: &str) -> IResult<&str, Token, ParseError<&str>> {
    alt((parse_variable, parse_brace, parse_text))(i)
}

pub fn parse_input(i: &str) -> anyhow::Result<Vec<Token>> {
    many0(parse_token)(i)
        .map(|(_, tokens)| tokens)
        .map_err(|e| anyhow::anyhow!("{}", e))
}
