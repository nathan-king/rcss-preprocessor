use crate::error::Span;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Rule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
    pub media: Vec<MediaBlock>,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug)]
pub struct MediaBlock {
    pub query: String,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
    pub variables: HashMap<String, String>,
}
