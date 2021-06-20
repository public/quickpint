use std::convert::{Infallible, TryFrom};

use pyo3::{exceptions::PyValueError, prelude::*, types::{PyFloat, PyLong}};
use rustpython_parser::token::Tok;

#[derive(Copy, Clone)]
pub enum TokenType {
    OP = 54,
    NUMBER = 2,
    NAME = 1,
    ENDMARKER = 0,
    IGNORE = -1,
}

impl From<i32> for TokenType {
    fn from(v: i32) -> Self {
        match v {
            x if x == TokenType::OP as i32 => TokenType::OP,
            x if x == TokenType::NUMBER as i32 => TokenType::NUMBER,
            x if x == TokenType::NAME as i32 => TokenType::NAME,
            x if x == TokenType::ENDMARKER as i32 => TokenType::ENDMARKER,
            _ => TokenType::IGNORE,
        }
    }
}

impl From<&Tok> for TokenType {
    fn from(v: &Tok) -> Self {
        match v {
            Tok::Plus | Tok::Minus | Tok::Star | Tok::Slash | Tok::DoubleStar | Tok::CircumFlex => {
                TokenType::OP
            }
            Tok::Int { value: _ } | Tok::Float { value: _ } | Tok::Complex { real: _, imag: _ } => {
                TokenType::NUMBER
            }
            Tok::Name { name: _ } => TokenType::NAME,
            Tok::In => TokenType::NAME,
            Tok::EndOfFile => TokenType::ENDMARKER,
            _ => TokenType::IGNORE,
        }
    }
}

impl Into<u16> for TokenType {
    fn into(self) -> u16 {
        self as u16
    }
}

#[pyclass]
#[derive(Clone)]
pub struct TokenInfo {
    pub type_id: TokenType,
    pub string: String,
}

impl TryFrom<&Tok> for TokenInfo {
    type Error = ();

    fn try_from(tok: &Tok) -> Result<Self, Self::Error> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let tok_string = match &tok {
            Tok::Name { name } => name.clone(),
            Tok::Int { value } => value.to_string(),
            Tok::Float { value } => PyFloat::new(py, *value).to_string(),
            Tok::Plus => "+".into(),
            Tok::Minus => "-".into(),
            Tok::Star => "*".into(),
            Tok::Slash => "/".into(),
            Tok::DoubleStar => "**".into(),
            Tok::CircumFlex => "^".into(),
            Tok::EndOfFile => "".into(),
            Tok::In => "in".into(),
            _ => return Err(()),
        };

        return Ok(TokenInfo {
            type_id: TokenType::from(tok),
            string: tok_string,
        });
    }
}


/* 
#[pyproto]
impl PySequenceProtocol for TokenInfo {
    fn __getitem__(&self, idx: isize) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        match idx {
            0 => Ok((self.type_id as u16).to_object(py)),
            1 => Ok(self.string.to_object(py)),
            _ => Err(PyIndexError::new_err(idx)),
        }
    }
}
*/