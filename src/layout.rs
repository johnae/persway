use anyhow::{anyhow, Result};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

impl FromStr for WorkspaceLayout {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "spiral" => Ok(Self::Spiral),
            "stack_main" => Ok(Self::StackMain),
            "manual" => Ok(Self::Manual),
            _ => Err(anyhow!("I don't know about the layout '{}'", s)),
        }
    }
}

impl Display for WorkspaceLayout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string_layout = match self {
            Self::Spiral => "spiral",
            Self::StackMain => "stack_main",
            Self::Manual => "manual",
        };
        write!(f, "{}", string_layout)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceLayout {
    Spiral,
    StackMain,
    Manual,
}
