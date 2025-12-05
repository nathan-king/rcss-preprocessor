use crate::error::Span;

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

pub type Stylesheet = Vec<Rule>;
