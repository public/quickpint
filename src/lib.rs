use std::convert::TryFrom;

mod eval_tree;
use crate::eval_tree::*;

mod tokenizer;
use crate::tokenizer::*;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use pyo3::wrap_pyfunction;

use rustpython_parser::lexer::make_tokenizer;

#[pyfunction]
pub fn tokenizer(_py: Python, expression: &str) -> PyResult<Vec<TokenInfo>> {
    let lexer = make_tokenizer(expression);
    let mut tokens: Vec<TokenInfo> = Vec::new();

    for step in lexer {
        match step {
            Ok((_, tok, _)) => match TokenInfo::try_from(&tok) {
                Ok(t) => tokens.push(t),
                Err(_) => continue,
            },
            Err(_) => return Err(PyValueError::new_err("tokenizer failure")),
        }
    }

    tokens.push(TokenInfo {
        r#type: TokenType::ENDMARKER,
        string: "".into(),
    });

    return Ok(tokens);
}

#[pyfunction]
fn build_eval_tree(py: Python, py_tokens: &PyList) -> PyResult<EvalTreeNode> {
    let mut tokens: Vec<TokenInfo> = Vec::new();
    tokens.reserve(py_tokens.len());

    for py_token in py_tokens {
        tokens.push(TokenInfo::try_from(py_token)?);
    }

    return Ok(*parse_tokens(py, &tokens, 0, 0, None)?.right);
}

#[pymodule]
fn quickpint(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_wrapped(wrap_pyfunction!(tokenizer))?;
    module.add_wrapped(wrap_pyfunction!(build_eval_tree))?;
    module.add_class::<EvalTreeNode>()?;
    Ok(())
}
