mod parser;

use parser::Token;

use anyhow::{Context as ErrorContext, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub type VariableKey = String;
pub type VariableValue = String;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Namespace {
    name: String,
    variables: HashMap<VariableKey, VariableValue>,
}

pub const GLOBAL_NS: &str = "GLOBAL";

impl Namespace {
    pub fn global() -> Self {
        Self {
            name: GLOBAL_NS.to_string(),
            variables: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SerializedContext {
    #[serde(default = "Namespace::global")]
    global: Namespace,
    #[serde(default)]
    renders: HashMap<PathBuf, PathBuf>,
    namespaces: Vec<Namespace>,
}

impl SerializedContext {
    pub fn to_context(self) -> Context {
        let mut namespaces: HashMap<String, Namespace> = self
            .namespaces
            .into_iter()
            .map(|ns| (ns.name.clone(), ns))
            .collect();
        let global = if let Some(global) = namespaces.remove(GLOBAL_NS) {
            global
        } else {
            self.global
        };
        Context {
            global,
            renders: self.renders,
            namespaces,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Context {
    global: Namespace,
    renders: HashMap<PathBuf, PathBuf>,
    namespaces: HashMap<String, Namespace>,
}

#[allow(dead_code)]
impl Context {
    fn get_namespace(&self, namespace: &str) -> Option<&Namespace> {
        self.namespaces.get(namespace)
    }

    fn global(&self) -> &Namespace {
        &self.global
    }

    fn get_global_variable(&self, key: &str) -> Option<&VariableValue> {
        self.global.variables.get(key)
    }

    fn get_variable_value(&self, key: &str, namespace: &str) -> Option<&VariableValue> {
        self.namespaces
            .get(namespace)
            .and_then(|ns| ns.variables.get(key))
            .or_else(|| self.get_global_variable(key))
    }

    pub fn renders(&self) -> &HashMap<PathBuf, PathBuf> {
        &self.renders
    }
}

#[derive(Debug, Default)]
pub struct Mold {
    context: Context,
}

impl Mold {
    pub fn new(context_file: &std::path::Path) -> Result<Self> {
        let data = std::fs::read(context_file).context("context file read error")?;
        serde_yaml::from_slice::<SerializedContext>(&data)
            .map(|ctx| Mold {
                context: ctx.to_context(),
            })
            .context("context deserialization error")
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn render(&self, input: &str, namespace: Option<&str>, render_raw: bool) -> Result<String> {
        let mut out = String::new();
        let tokens = parser::parse_input(&input).context("parsing input error")?;
        for token in tokens {
            match token {
                Token::Text(t) => out.push_str(t),
                Token::Variable { name, raw } => {
                    let rendered = if let Some(ns) = namespace {
                        if let Some(value) = self.context.get_variable_value(name, ns) {
                            // try to render variable in case it contains nested variables
                            if let Ok(rendered) = self.render(value.as_str(), namespace, render_raw)
                            {
                                out.push_str(&rendered);
                            } else {
                                out.push_str(&value);
                            }
                            true
                        } else {
                            false
                        }
                    } else {
                        // try to use variables from global namespace
                        if let Some(value) = self.context.get_global_variable(name) {
                            if let Ok(rendered) = self.render(value.as_str(), namespace, render_raw)
                            {
                                out.push_str(&rendered);
                            } else {
                                out.push_str(&value);
                            }
                            true
                        } else {
                            false
                        }
                    };
                    if !rendered && render_raw {
                        out.push_str(&raw);
                    }
                }
            }
        }

        Ok(out)
    }

    pub fn render_file(
        &self,
        file: &std::path::Path,
        namespace: Option<&str>,
        render_raw: bool,
    ) -> Result<String> {
        let input = std::fs::read_to_string(file).context("render file read error")?;
        self.render(&input, namespace, render_raw)
    }
}
