#[derive(Debug)]
pub struct Rule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug)]
pub struct Declaration {
    pub property: String,
    pub value: String,
}

pub type Stylesheet = Vec<Rule>;
