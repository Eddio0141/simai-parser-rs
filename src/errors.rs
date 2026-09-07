use std::{fmt::Display, num::NonZeroU8};

use nom::error::{FromExternalError, ParseError};

// #[derive(Debug)]
// pub enum Error<I> {
//     User(Box<dyn std::error::Error + Send + Sync + 'static>),
//     External {
//         input: I,
//         code: nom::error::ErrorKind,
//         err: Box<dyn std::error::Error + Send + Sync + 'static>,
//     },
//     Nom {
//         input: I,
//         code: nom::error::ErrorKind,
//     },
// }
//
// #[derive(Debug)]
// pub struct ErrorStack<I>(pub Vec<Error<I>>);
//
// impl<I> ParseError<I> for ErrorStack<I> {
//     fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
//         Self(vec![Error::Nom { input, code: kind }])
//     }
//
//     fn append(input: I, kind: nom::error::ErrorKind, mut other: Self) -> Self {
//         other.0.push(Error::Nom { input, code: kind });
//         other
//     }
// }
//
// impl<I, E: std::error::Error + Send + Sync + 'static> FromExternalError<I, E> for ErrorStack<I> {
//     fn from_external_error(input: I, kind: nom::error::ErrorKind, e: E) -> Self {
//         Self(vec![Error::External {
//             input,
//             code: kind,
//             err: Box::new(e),
//         }])
//     }
// }
//
// impl<I, E: std::error::Error + Send + Sync + 'static> From<E> for ErrorStack<I> {
//     fn from(value: E) -> Self {
//         Self(vec![Error::User(Box::new(value))])
//     }
// }

#[derive(Debug)]
pub struct DuplicateFieldError(pub &'static str);

impl Display for DuplicateFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Duplicate map field `{}`", self.0)
    }
}

impl core::error::Error for DuplicateFieldError {}
