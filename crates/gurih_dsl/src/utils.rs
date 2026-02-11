use crate::errors::CompileError;
pub use gurih_ir::utils::{capitalize, to_title_case};
use kdl::{KdlNode, KdlValue};

pub fn get_arg_string(node: &KdlNode, index: usize, src: &str) -> Result<String, CompileError> {
    node.entry(index)
        .and_then(|val| val.value().as_string().map(|s| s.to_string()))
        .ok_or_else(|| CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: format!(
                "Missing or invalid argument at index {} for node '{}'",
                index,
                node.name().value()
            ),
        })
}

pub fn get_prop_string(node: &KdlNode, key: &str, src: &str) -> Result<String, CompileError> {
    node.get(key)
        .and_then(|val| val.as_string().map(|s| s.to_string()))
        .ok_or_else(|| CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: format!("Missing property '{}'", key),
        })
}

fn parse_bool_value(val: &KdlValue) -> Option<bool> {
    if let Some(b) = val.as_bool() {
        Some(b)
    } else if let Some(s) = val.as_string() {
        match s {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    } else {
        None
    }
}

pub fn get_prop_bool(node: &KdlNode, key: &str) -> Option<bool> {
    node.get(key).and_then(parse_bool_value)
}

pub fn get_prop_int(node: &KdlNode, key: &str, src: &str) -> Result<i64, CompileError> {
    node.get(key)
        .and_then(|val| val.as_integer().map(|i| i as i64))
        .ok_or_else(|| CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: format!("Missing or invalid int property '{}'", key),
        })
}

pub fn get_arg_bool(node: &KdlNode, index: usize) -> Result<bool, CompileError> {
    node.entry(index)
        .and_then(|val| parse_bool_value(val.value()))
        .ok_or_else(|| CompileError::ParseError {
            src: "".to_string(),
            span: node.span().into(),
            message: format!("Missing or invalid bool argument at index {}", index),
        })
}
