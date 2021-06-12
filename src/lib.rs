use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::types::PyList;
use pyo3::exceptions::PyValueError;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::fmt;

#[pyclass]
struct EvalTreeNode {
    left: Option<Box<EvalTreeNode>>,
    operator: Option<Token>,
    right: Option<Box<EvalTreeNode>>
}

impl fmt::Display for EvalTreeNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.right {
            Some(r) => {
                assert!(self.left.is_some());

                let comps;

                if self.operator.is_some() {
                    comps = vec![self.left.as_ref().unwrap().to_string(), self.operator.as_ref().unwrap().value.clone(), r.to_string()];
                } else {
                    comps = vec![self.left.as_ref().unwrap().to_string(), r.to_string()];
                }
    
                return write!(f, "({comps})", comps=comps.join(" "))
            },

            None if self.operator.is_some() => {
                let comps;

                if self.left.is_some() {
                    comps = vec![self.operator.as_ref().unwrap().value.clone(), self.left.as_ref().unwrap().to_string()];
                } else {
                    comps = vec![self.operator.as_ref().unwrap().value.clone()];
                }
                
                return write!(f, "{comps}", comps=comps.join(" "))
            },

            _ => {
                return write!(f, "{value}", value=self.operator.as_ref().unwrap().value.clone());
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
    typeID: TokenType,
    value: String,
}

struct ParseStep {
    right: Box<EvalTreeNode>,
    index: usize,
}

fn op_priority(op: &str) -> i16 {
     match op {
        "**" | "^"=> 3,
        "unary" => 2,
        "*" | "" | "/" => 1,
        "+" | "-" => 0,
        _ => -1
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

fn parse_tokens(_py: Python, tokens: &PyList, mut index: usize, depth: i64, prev_op: Option<&str>) -> PyResult<ParseStep> {
    let prevOpPriority = match prev_op {
        Some(op) => op_priority(op),
        None => -1,
    };

    let mut result: Option<Box<EvalTreeNode>> = None;

    while index < tokens.len() {
        let pyToken = tokens.get_item(index.try_into()?);

        println!("index={index} token={token} result={result}", index=index, token=pyToken, result=result.is_some());

        let typeID: i32 = pyToken.get_item(0)?.extract()?;
        let value: String = pyToken.get_item(1)?.extract()?;

        let token = Token {
            typeID: TokenType::try_from(typeID)?,
            value: value,
        };

        match token.typeID {
            TokenType::OP => match token.value.as_str() {

                ")" => match prev_op {
                    None => return Err(PyValueError::new_err("unopened parenthesis")),
                    Some("(") => return Ok(ParseStep{right: result.unwrap(), index: index }),
                    _ => return Ok(ParseStep{right: result.unwrap(), index: index - 1}),
                },

                "(" => {
                    let step = parse_tokens(_py, tokens, index + 1, 0, Some(token.value.as_str()))?;
                    index = step.index;

                    let lastTokenValue: &str = tokens.get_item(index.try_into()?).get_item(1)?.extract()?;

                    if lastTokenValue != ")" {
                        return Err(PyValueError::new_err("weird exit from parenthesis")) 
                    }

                    if result.is_some() {
                        result = Some(Box::new(EvalTreeNode { left: result, operator: None, right: Some(step.right) }));
                    } else {
                        result = Some(step.right);
                    }
                },

                _ => {
                    let opPriority = op_priority(token.value.as_str());
                    
                    if result.is_some() {
                        if opPriority < prevOpPriority && token.value != "**" && token.value != "^" {
                            return Ok(ParseStep{right: result.unwrap(), index: index - 1});
                        } else {
                            let step = parse_tokens(_py, tokens, index + 1, depth + 1, Some(token.value.as_str()))?;
                            println!("OP {op}", op=token.value);
                            result = Some(Box::new(EvalTreeNode { left: result, operator: Some(token), right: Some(step.right) }));
                            index = step.index;
                        }
                    } else {
                        let step = parse_tokens(_py, tokens, index + 1, depth + 1, Some("unary"))?;

                        result = Some(Box::new(EvalTreeNode { left: Some(step.right), operator: Some(token), right: None }));
                        index = step.index;
                    }
                }
            },

            TokenType::NUMBER | TokenType::NAME => match result {
                Some(tree) => if op_priority("") <= prevOpPriority {
                    return Ok(ParseStep{right: tree, index: index - 1});
                } else {
                    let step = parse_tokens(_py, tokens, index, depth + 1, Some(""))?;

                    result = Some(Box::new(EvalTreeNode { left: Some(tree), operator: None, right: Some(step.right) }));
                    index = step.index;
                },
                None => {
                    result = Some(Box::new(EvalTreeNode { left: None, operator: Some(token), right: None }))
                },
            },

            TokenType::ENDMARKER => match prev_op {
                Some("(") => return Err(PyValueError::new_err("unclosed parenthesis")),
                Some(_) => return Ok(ParseStep{right: result.unwrap(), index: index}),
                _ => if depth > 0 {
                    return Ok(ParseStep{right: result.unwrap(), index: index})
                } else {
                    return Ok(ParseStep{right: result.unwrap(), index: 0})
                }
            },

            TokenType::IGNORE => ()
        };

        index += 1;
    }

    let done = result.unwrap();

    println!("result={done}", done=done);

    return Ok(ParseStep{right: done, index: index})
}

#[pyfunction]
fn build_eval_tree(_py: Python, tokens: &PyList) -> PyResult<EvalTreeNode> {
    println!("build_eval_tree");

    return Ok(*parse_tokens(_py, tokens, 0, 0, None)?.right);
}


#[pymodule]
fn quickpint(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(build_eval_tree, m)?)?;
    m.add_class::<EvalTreeNode>()?;

    Ok(())
}