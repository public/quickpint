use pyo3::exceptions::PyValueError;
use pyo3::ffi::PyFloat_Type;
use pyo3::ffi::PyNumber_Absolute;
use pyo3::ffi::PyNumber_Add;
use pyo3::ffi::PyNumber_Multiply;
use pyo3::ffi::PyNumber_Negative;
use pyo3::ffi::PyNumber_Positive;
use pyo3::ffi::PyNumber_Power;
use pyo3::ffi::PyNumber_Subtract;
use pyo3::ffi::PyNumber_TrueDivide;
use pyo3::ffi::Py_None;
use pyo3::prelude::*;
use pyo3::types::PyFloat;
use pyo3::types::PyList;
use pyo3::types::PyString;
use pyo3::types::PyTuple;
use pyo3::wrap_pyfunction;
use pyo3::AsPyPointer;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::fmt;
use std::u8;

#[pyclass]
struct EvalTreeNode {
    left: Option<Box<EvalTreeNode>>,
    operator: Option<Token>,
    right: Option<Box<EvalTreeNode>>,
}

impl fmt::Display for EvalTreeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.right {
            Some(r) => match &self.left {
                Some(l) => {
                    return write!(
                        f,
                        "({comps})",
                        comps = match &self.operator {
                            Some(op) => vec![l.to_string(), op.string.clone(), r.to_string()],
                            None => vec![l.to_string(), r.to_string()],
                        }
                        .join(" ")
                    )
                }
                None => return Err(std::fmt::Error),
            },

            None if self.operator.is_some() => {
                let op = self.operator.as_ref().unwrap();
                let comps = match &self.left {
                    Some(l) => vec![op.string.clone(), l.to_string()],
                    None => vec![op.string.clone()],
                };

                return write!(f, "{comps}", comps = comps.join(" "));
            }

            _ => {
                let op = self.operator.as_ref().unwrap();
                return write!(f, "{value}", value = op.string.clone());
            }
        }
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
                let args = PyTuple::new(py, &[op.original.as_ref(py)]);
                Ok(py_callback.call1(args)?.into())
            }
            _ => Err(PyValueError::new_err("unable to evaluate tree")),
        };
    }
}

#[derive(Clone)]
struct Token {
    type_id: TokenType,
    string: String,
    original: PyObject,
}

impl Token {
    fn binary(&self, py: Python, left: PyObject, right: PyObject) -> PyResult<PyObject> {
        let left_ptr = left.as_ptr();
        let right_ptr = right.as_ptr();

        unsafe {
            let none_ptr = Py_None();

            Ok(PyObject::from_owned_ptr(
                py,
                match self.string.as_str() {
                    "**" => PyNumber_Power(left_ptr, right_ptr, none_ptr),
                    "*" | "" => PyNumber_Multiply(left_ptr, right_ptr),
                    "/" => PyNumber_TrueDivide(left_ptr, right_ptr),
                    "+" => PyNumber_Add(left_ptr, right_ptr),
                    "-" => PyNumber_Subtract(left_ptr, right_ptr),
                    _ => panic!("unknown binary op"),
                },
            ))
        }
    }

    fn unary(&self, py: Python, left: PyObject) -> PyResult<PyObject> {
        let left_ptr = left.as_ptr();

        unsafe {
            Ok(PyObject::from_owned_ptr(
                py,
                match self.string.as_str() {
                    "+" => PyNumber_Positive(left_ptr),
                    "-" => PyNumber_Negative(left_ptr),
                    _ => panic!("unknown unary op"),
                },
            ))
        }
    }
}

struct ParseStep {
    right: Box<EvalTreeNode>,
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

#[derive(Copy, Clone)]
enum TokenType {
    OP = 54,
    NUMBER = 2,
    NAME = 1,
    ENDMARKER = 0,
    IGNORE = -1,
}

impl TryFrom<i32> for TokenType {
    type Error = PyErr;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == TokenType::OP as i32 => Ok(TokenType::OP),
            x if x == TokenType::NUMBER as i32 => Ok(TokenType::NUMBER),
            x if x == TokenType::NAME as i32 => Ok(TokenType::NAME),
            x if x == TokenType::ENDMARKER as i32 => Ok(TokenType::ENDMARKER),
            _ => Ok(TokenType::IGNORE),
        }
    }
}

fn parse_tokens(
    py: Python,
    tokens: &Vec<Token>,
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
                        if op_priority < prev_op_priority
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

    let done = result.unwrap();

    return Ok(ParseStep {
        right: done,
        index: index,
    });
}

#[pyfunction]
fn build_eval_tree(py: Python, py_tokens: &PyList) -> PyResult<EvalTreeNode> {
    let mut tokens = Vec::new();
    tokens.reserve(py_tokens.len());

    for py_token in py_tokens {
        let type_id: i32 = py_token.get_item(0)?.extract()?;
        let value: String = py_token.get_item(1)?.extract()?;

        let token = Token {
            type_id: TokenType::try_from(type_id)?,
            string: value,
            original: py_token.to_object(py),
        };

        tokens.push(token);
    }

    return Ok(*parse_tokens(py, &tokens, 0, 0, None)?.right);
}

#[pymodule]
fn quickpint(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(build_eval_tree, m)?)?;
    m.add_class::<EvalTreeNode>()?;

    Ok(())
}
