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

static FILE_START_TAG: &str = "{@";
static FILE_END_TAG: &str = "@}";
static FILE_TRIM_START_TAG: &str = "{@~";
static FILE_TRIM_END_TAG: &str = "~@}";
static VAR_START_TAG: &str = "{%";
static VAR_END_TAG: &str = "%}";

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
    FileSource { path: &'a str, trim: bool },
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

fn file_path_impl<'a>(
    i: &'a str,
    end_tag: &'static str,
) -> IResult<&'a str, &'a str, ParseError<&'a str>> {
    if let Some(pos) = i.find(end_tag) {
        let trimmed = i.split(end_tag).next().unwrap().trim();

        Ok((&i[pos..], trimmed))
    } else {
        Err(nom::Err::Failure(ParseError::Nom(i, ErrorKind::Verify)))
    }
}

fn file_path(i: &str) -> IResult<&str, &str, ParseError<&str>> {
    file_path_impl(i, FILE_END_TAG)
}

fn file_path_trim(i: &str) -> IResult<&str, &str, ParseError<&str>> {
    file_path_impl(i, FILE_TRIM_END_TAG)
}

fn space_count(i: &str) -> IResult<&str, usize, ParseError<&str>> {
    many0_count(char(' '))(i)
}

fn parse_enclosed_tag<'a>(
    i: &'a str,
    start_tag: &'static str,
    end_tag: &'static str,
    take_while: impl FnMut(&'a str) -> IResult<&'a str, &'a str, ParseError<&'a str>>,
    f: impl FnMut((usize, &'a str, usize)) -> Token<'a>,
) -> IResult<&'a str, Token<'a>, ParseError<&'a str>> {
    map(
        tuple((
            preceded(tag(start_tag), space_count),
            take_while,
            terminated(space_count, tag(end_tag)),
        )),
        f,
    )(i)
}

fn parse_variable(i: &str) -> IResult<&str, Token, ParseError<&str>> {
    parse_enclosed_tag(
        i,
        VAR_START_TAG,
        VAR_END_TAG,
        variable_name,
        |(count1, name, count2)| Token::Variable {
            name,
            raw: &i[..name.len() + 4 + count1 + count2],
        },
    )
}

fn parse_file_source(i: &str) -> IResult<&str, Token, ParseError<&str>> {
    parse_enclosed_tag(
        i,
        FILE_START_TAG,
        FILE_END_TAG,
        file_path,
        |(_, path, _)| Token::FileSource { path, trim: false },
    )
}

fn parse_file_source_trim(i: &str) -> IResult<&str, Token, ParseError<&str>> {
    parse_enclosed_tag(
        i,
        FILE_TRIM_START_TAG,
        FILE_TRIM_END_TAG,
        file_path_trim,
        |(_, path, _)| Token::FileSource { path, trim: true },
    )
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
    alt((
        parse_variable,
        parse_file_source_trim,
        parse_file_source,
        parse_brace,
        parse_text,
    ))(i)
}

pub fn parse_input(i: &str) -> anyhow::Result<Vec<Token>> {
    many0(parse_token)(i)
        .map(|(_, tokens)| tokens)
        .map_err(|e| anyhow::anyhow!("{}", e))
}
