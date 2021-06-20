
use pyo3::ffi::PyNumber_Add;
use pyo3::ffi::PyNumber_Multiply;
use pyo3::ffi::PyNumber_Negative;
use pyo3::ffi::PyNumber_Positive;
use pyo3::ffi::PyNumber_Power;
use pyo3::ffi::PyNumber_Subtract;
use pyo3::ffi::PyNumber_TrueDivide;
use pyo3::ffi::Py_None;

use pyo3::prelude::*;
use core::fmt;
use core::panic;

use pyo3::exceptions::{PyValueError};

use pyo3::{AsPyPointer};
use crate::tokenizer::TokenInfo;
use crate::tokenizer::TokenType;


#[pyclass]
pub struct EvalTreeNode {
    pub left: Option<Box<EvalTreeNode>>,
    pub operator: Option<TokenInfo>,
    pub right: Option<Box<EvalTreeNode>>,
}

impl fmt::Display for EvalTreeNode {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let parts = match self {
            EvalTreeNode {
                left: Some(l),
                operator: Some(op),
                right: Some(r),
            } => vec![l.to_string(), op.string.clone(), r.to_string()],

            EvalTreeNode {
                left: Some(l),
                operator: None,
                right: Some(r),
            } => vec![l.to_string(), r.to_string()],

            EvalTreeNode {
                left: Some(l),
                operator: Some(op),
                right: None,
            } => vec![op.string.clone(), l.to_string()],

            EvalTreeNode {
                left: None,
                operator: Some(op),
                right: None,
            } => vec![op.string.clone()],

            _ => panic!("unexpected tree node"),
        };

        let value = parts.join(" ");

        return match parts.len() {
            1 => write!(out, "{value}", value = value),
            _ => write!(out, "({value})", value = value),
        };
    }
}

#[pymethods]
impl EvalTreeNode {
    fn evaluate(&self, py: Python, py_callback: &PyAny) -> PyResult<PyObject> {
        return match self {
            EvalTreeNode {
                left: Some(l),
                operator: Some(op),
                right: Some(r),
            } => op.binary(
                py,
                l.evaluate(py, py_callback)?.into(),
                r.evaluate(py, py_callback)?.into(),
            ),
            EvalTreeNode {
                left: Some(l),
                operator: Some(op),
                right: _,
            } => op.unary(py, l.evaluate(py, py_callback)?.into()),
            EvalTreeNode {
                left: _,
                operator: Some(op),
                right: _,
            } => {
                Ok(py_callback.call1((op.clone(),))?.into())
            }
            _ => Err(PyValueError::new_err("unable to evaluate tree")),
        };
    }

    fn to_string(&self) -> PyResult<String> {
        return Ok(ToString::to_string(&self));
    }
}

impl TokenInfo {
    fn binary(&self, py: Python, left: PyObject, right: PyObject) -> PyResult<PyObject> {
        let left_ptr = left.as_ptr();
        let right_ptr = right.as_ptr();

        unsafe {
            let none_ptr = Py_None();

            let result_ptr = match self.string.as_str() {
                "**" | "^" => PyNumber_Power(left_ptr, right_ptr, none_ptr),
                "*" | "" => PyNumber_Multiply(left_ptr, right_ptr),
                "/" => PyNumber_TrueDivide(left_ptr, right_ptr),
                "+" => PyNumber_Add(left_ptr, right_ptr),
                "-" => PyNumber_Subtract(left_ptr, right_ptr),
                _ => panic!("unknown binary op"),
            };

            return PyObject::from_owned_ptr_or_err(py, result_ptr);
        }
    }

    fn unary(&self, py: Python, left: PyObject) -> PyResult<PyObject> {
        let left_ptr = left.as_ptr();
        unsafe {
            return PyObject::from_owned_ptr_or_err(
                py,
                match self.string.as_str() {
                    "+" => PyNumber_Positive(left_ptr),
                    "-" => PyNumber_Negative(left_ptr),
                    _ => panic!("unknown unary op"),
                },
            );
        }
    }
}

pub struct ParseStep {
    pub right: Box<EvalTreeNode>,
    index: isize,
}

fn op_priority(op: &str) -> i16 {
    match op {
        "**" | "^" => 3,
        "unary" => 2,
        "*" | "" | "/" => 1,
        "+" | "-" => 0,
        _ => -1,
    }
}

pub fn parse_tokens(
    py: Python,
    tokens: &Vec<TokenInfo>,
    mut index: isize,
    depth: i64,
    prev_op: Option<&str>,
) -> PyResult<ParseStep> {
    let prev_op_priority = match prev_op {
        Some(op) => op_priority(op),
        None => -1,
    };

    let mut result: Option<Box<EvalTreeNode>> = None;

    let len_tokens = tokens.len() as isize;

    while index < len_tokens {
        let token = &tokens[index as usize];

        match token.type_id {
            TokenType::OP => match token.string.as_str() {
                ")" => match prev_op {
                    None => return Err(PyValueError::new_err("unopened parenthesis")),
                    Some("(") => {
                        return Ok(ParseStep {
                            right: result.unwrap(),
                            index: index,
                        })
                    }
                    _ => {
                        return Ok(ParseStep {
                            right: result.unwrap(),
                            index: index - 1,
                        })
                    }
                },

                "(" => {
                    let step = parse_tokens(py, tokens, index + 1, 0, Some(token.string.as_str()))?;
                    index = step.index;

                    let last_token_value: &str = tokens[index as usize].string.as_str();

                    if last_token_value != ")" {
                        return Err(PyValueError::new_err("weird exit from parenthesis"));
                    }

                    if result.is_some() {
                        result = Some(Box::new(EvalTreeNode {
                            left: result,
                            operator: None,
                            right: Some(step.right),
                        }));
                    } else {
                        result = Some(step.right);
                    }
                }

                _ => {
                    let op_priority = op_priority(token.string.as_str());

                    if result.is_some() {
                        if op_priority <= prev_op_priority
                            && token.string != "**"
                            && token.string != "^"
                        {
                            return Ok(ParseStep {
                                right: result.unwrap(),
                                index: index - 1,
                            });
                        } else {
                            let step = parse_tokens(
                                py,
                                tokens,
                                index + 1,
                                depth + 1,
                                Some(token.string.as_str()),
                            )?;

                            result = Some(Box::new(EvalTreeNode {
                                left: result,
                                operator: Some(token.clone()),
                                right: Some(step.right),
                            }));
                            index = step.index;
                        }
                    } else {
                        let step = parse_tokens(py, tokens, index + 1, depth + 1, Some("unary"))?;

                        result = Some(Box::new(EvalTreeNode {
                            left: Some(step.right),
                            operator: Some(token.clone()),
                            right: None,
                        }));
                        index = step.index;
                    }
                }
            },

            TokenType::NUMBER | TokenType::NAME => match result {
                Some(tree) => {
                    if op_priority("") <= prev_op_priority {
                        return Ok(ParseStep {
                            right: tree,
                            index: index - 1,
                        });
                    } else {
                        let step = parse_tokens(py, tokens, index, depth + 1, Some(""))?;

                        result = Some(Box::new(EvalTreeNode {
                            left: Some(tree),
                            operator: None,
                            right: Some(step.right),
                        }));
                        index = step.index;
                    }
                }
                None => {
                    result = Some(Box::new(EvalTreeNode {
                        left: None,
                        operator: Some(token.clone()),
                        right: None,
                    }))
                }
            },

            TokenType::ENDMARKER => match prev_op {
                Some("(") => return Err(PyValueError::new_err("unclosed parenthesis")),
                Some(_) => {
                    return Ok(ParseStep {
                        right: result.unwrap(),
                        index: index,
                    })
                }
                _ => {
                    if depth > 0 {
                        return Ok(ParseStep {
                            right: result.unwrap(),
                            index: index,
                        });
                    } else {
                        return Ok(ParseStep {
                            right: result.unwrap(),
                            index: 0,
                        });
                    }
                }
            },

            TokenType::IGNORE => (),
        };

        index += 1;
    }

    return match result {
        Some(value) => Ok(ParseStep {
            right: value,
            index: index,
        }),
        None => Err(PyValueError::new_err("no result?")),
    };
}

