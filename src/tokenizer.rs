use std::convert::{TryFrom};

use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3::{PySequenceProtocol, exceptions::{PyIndexError}, types::{PyFloat}};
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


impl<'source> FromPyObject<'source> for TokenType {
    fn extract(ob: &'source PyAny) -> PyResult<TokenType> {
        let v: i32 = ob.extract()?;
        Ok(TokenType::from(v))
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

impl IntoPy<PyObject> for TokenType {
    fn into_py(self, py: Python) -> PyObject {
        (self as u16).into_py(py)
    }
}

#[pyclass]
#[derive(Clone)]
pub struct TokenInfo {
    #[pyo3(get, set)]
    pub r#type: TokenType,
    #[pyo3(get, set)]
    pub string: String,
}

impl TryFrom<&PyAny> for TokenInfo {
    type Error = PyErr;

    fn try_from(tok: &PyAny) -> Result<Self, Self::Error> {
        Ok(TokenInfo{
            r#type: tok.get_item(0)?.extract()?,
            string: tok.get_item(1)?.extract()?
        })
    }
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
            r#type: TokenType::from(tok),
            string: tok_string,
        });
    }
}

#[pyproto]
impl PySequenceProtocol for TokenInfo {
    fn __getitem__(&self, idx: isize) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        match idx {
            0 => Ok((self.r#type as u16).to_object(py)),
            1 => Ok(self.string.to_object(py)),
            _ => Err(PyIndexError::new_err(idx)),
        }
    }
}