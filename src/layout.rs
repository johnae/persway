use anyhow::{anyhow, Result};
use std::{
    fmt::{Display, Formatter},
    ops::RangeInclusive,
    str::FromStr,
};

pub const STACK_MAIN_DEFAULT_SIZE: u8 = 70;

impl FromStr for WorkspaceLayout {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "spiral" => Ok(Self::Spiral),
            "stack_main" => Ok(Self::StackMain {
                stack_layout: StackLayout::Stacked,
                size: STACK_MAIN_DEFAULT_SIZE,
            }),
            "manual" => Ok(Self::Manual),
            s => Err(anyhow!("I don't know about the layout '{}'", s)),
        }
    }
}

impl Display for WorkspaceLayout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string_layout = match self {
            Self::Spiral => String::from("spiral"),
            Self::StackMain { stack_layout, size } => {
                format!("stack_main {} {}", stack_layout, size)
            }
            Self::Manual => String::from("manual"),
        };
        write!(f, "{}", string_layout)
    }
}

const SIZE_RANGE: RangeInclusive<usize> = 10..=90;

fn size_in_range(s: &str) -> Result<u8, String> {
    let size: usize = s.parse().map_err(|_| format!("{s} is not a valid size"))?;
    if SIZE_RANGE.contains(&size) {
        return Ok(size as u8);
    }
    Err(format!(
        "size not in range {}-{}",
        SIZE_RANGE.start(),
        SIZE_RANGE.end()
    ))
}

impl FromStr for StackLayout {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "tabbed" => Ok(Self::Tabbed),
            "stacked" => Ok(Self::Stacked),
            "tiled" => Ok(Self::Tiled),
            s => Err(anyhow!("I don't know about the stack layout '{}'", s)),
        }
    }
}

impl Display for StackLayout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string_layout = match self {
            Self::Tabbed => "tabbed",
            Self::Stacked => "stacked",
            Self::Tiled => "tiled",
        };
        write!(f, "{}", string_layout)
    }
}

#[derive(clap::Parser, Debug, Clone, PartialEq)]
pub enum StackLayout {
    Tabbed,
    Stacked,
    Tiled,
}

#[derive(clap::Parser, Debug, Clone, PartialEq)]
pub enum WorkspaceLayout {
    /// The spiral autotiling layout tiles windows in a spiral formation, similar to AwesomeWM
    Spiral,
    /// The stack_main autotiling layout keeps a stack of windows on the side of a larger main area, this layout comes with a few commands to control it as well
    StackMain {
        /// Size of the main area in percent
        #[arg(long, short = 's', value_parser = size_in_range, default_value_t = STACK_MAIN_DEFAULT_SIZE)]
        size: u8,
        /// The sway layout of the stack: tabbed, tiled or stacked.
        #[arg(long, short = 'l', default_value_t = StackLayout::Stacked)]
        stack_layout: StackLayout,
    },
    /// The standard sway manual tiling
    Manual,
}
