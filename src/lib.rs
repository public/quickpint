use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::wrap_pyfunction;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::fmt;

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
                            Some(op) => vec![l.to_string(), op.value.clone(), r.to_string()],
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
                    Some(l) => vec![op.value.clone(), l.to_string()],
                    None => vec![op.value.clone()],
                };

                return write!(f, "{comps}", comps = comps.join(" "));
            }

            _ => {
                let op = self.operator.as_ref().unwrap();
                return write!(f, "{value}", value = op.value.clone());
            }
        }
    }
}

#[pymethods]
impl EvalTreeNode {
    fn evaluate(&self, callback: PyObject) -> PyResult<()> {
        Ok(())
    }
}

struct Token {
    type_id: TokenType,
    value: String,
}

struct ParseStep {
    right: Box<EvalTreeNode>,
    index: usize,
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
    _py: Python,
    tokens: &PyList,
    mut index: usize,
    depth: i64,
    prev_op: Option<&str>,
) -> PyResult<ParseStep> {
    let prev_op_priority = match prev_op {
        Some(op) => op_priority(op),
        None => -1,
    };

    let mut result: Option<Box<EvalTreeNode>> = None;

    while index < tokens.len() {
        let py_token = tokens.get_item(index.try_into()?);

        println!(
            "index={index} token={token} result={result}",
            index = index,
            token = py_token,
            result = result.is_some()
        );

        let type_id: i32 = py_token.get_item(0)?.extract()?;
        let value: String = py_token.get_item(1)?.extract()?;

        let token = Token {
            type_id: TokenType::try_from(type_id)?,
            value: value,
        };

        match token.type_id {
            TokenType::OP => match token.value.as_str() {
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
                    let step = parse_tokens(_py, tokens, index + 1, 0, Some(token.value.as_str()))?;
                    index = step.index;

                    let last_token_value: &str =
                        tokens.get_item(index.try_into()?).get_item(1)?.extract()?;

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
                    let op_priority = op_priority(token.value.as_str());

                    if result.is_some() {
                        if op_priority < prev_op_priority && token.value != "**" && token.value != "^"
                        {
                            return Ok(ParseStep {
                                right: result.unwrap(),
                                index: index - 1,
                            });
                        } else {
                            let step = parse_tokens(
                                _py,
                                tokens,
                                index + 1,
                                depth + 1,
                                Some(token.value.as_str()),
                            )?;
                            println!("OP {op}", op = token.value);
                            result = Some(Box::new(EvalTreeNode {
                                left: result,
                                operator: Some(token),
                                right: Some(step.right),
                            }));
                            index = step.index;
                        }
                    } else {
                        let step = parse_tokens(_py, tokens, index + 1, depth + 1, Some("unary"))?;

                        result = Some(Box::new(EvalTreeNode {
                            left: Some(step.right),
                            operator: Some(token),
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
                        let step = parse_tokens(_py, tokens, index, depth + 1, Some(""))?;

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
                        operator: Some(token),
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

    println!("result={done}", done = done);

    return Ok(ParseStep {
        right: done,
        index: index,
    });
}

#[pyfunction]
fn build_eval_tree(_py: Python, tokens: &PyList) -> PyResult<EvalTreeNode> {
    println!("build_eval_tree");

    return Ok(*parse_tokens(_py, tokens, 0, 0, None)?.right);
}

#[pymodule]
fn quickpint(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(build_eval_tree, m)?)?;
    m.add_class::<EvalTreeNode>()?;

    Ok(())
}
