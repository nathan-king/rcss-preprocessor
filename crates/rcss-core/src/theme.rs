use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Theme {
    pub collections: HashMap<String, Value>,
    pub properties: HashMap<String, PropertyMapping>,
    #[serde(default)]
    pub shorthands: HashMap<String, ShorthandDef>,
}

#[derive(Debug, Deserialize)]
pub struct PropertyMapping {
    pub collection: String,
    #[serde(default)]
    pub overrides: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct ShorthandStep {
    pub property: String,
    pub template: String,
    #[serde(default)]
    pub append: bool,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Deserialize)]
pub struct ShorthandDef {
    pub steps: Vec<ShorthandStep>,
    pub order: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawShorthand {
    Steps(Vec<ShorthandStep>),
    Object {
        steps: Vec<ShorthandStep>,
        #[serde(default)]
        order: Vec<String>,
    },
}

impl Theme {
    pub fn load_from_dir(dir: &str) -> Result<Self, String> {
        let path = format!("{}/tokens.json", dir);
        let mut theme = Self::load(&path)?;

        // Load shorthands if present
        let shorthand_path = format!("{}/shorthands.json", dir);
        if let Ok(data) = fs::read_to_string(&shorthand_path) {
            let raw: serde_json::Value = serde_json::from_str(&data)
                .map_err(|e| format!("Invalid JSON in {}: {}", shorthand_path, e))?;
            let mut shorthands = HashMap::new();

            let obj = raw.as_object().ok_or_else(|| {
                format!(
                    "Invalid JSON in {}: root should be an object",
                    shorthand_path
                )
            })?;

            for (key, val) in obj {
                let parsed: RawShorthand = serde_json::from_value(val.clone())
                    .map_err(|e| format!("Invalid shorthand '{}': {}", key, e))?;

                let def = match parsed {
                    RawShorthand::Steps(steps) => ShorthandDef { steps, order: None },
                    RawShorthand::Object { steps, order } => {
                        let order = if order.is_empty() { None } else { Some(order) };
                        ShorthandDef { steps, order }
                    }
                };

                shorthands.insert(key.clone(), def);
            }

            theme.shorthands = shorthands;
        }

        Ok(theme)
    }

    pub fn load(path: &str) -> Result<Self, String> {
        let data =
            fs::read_to_string(path).map_err(|e| format!("Could not read {}: {}", path, e))?;

        serde_json::from_str(&data).map_err(|e| format!("Invalid JSON in {}: {}", path, e))
    }
}
