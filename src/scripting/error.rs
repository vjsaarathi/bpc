//! Error types for the scripting layer.

use std::fmt;

/// Errors from the scripting engine.
#[derive(Debug)]
pub enum ScriptError {
    /// An error from the Lua runtime.
    Lua(mlua::Error),
    /// A layout-related error surfaced through scripting.
    Layout(crate::layout::LayoutError),
    /// A general error message.
    Message(String),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lua(e) => write!(f, "lua error: {e}"),
            Self::Layout(e) => write!(f, "layout error: {e}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ScriptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lua(e) => Some(e),
            Self::Layout(e) => Some(e),
            Self::Message(_) => None,
        }
    }
}

impl From<mlua::Error> for ScriptError {
    fn from(e: mlua::Error) -> Self {
        Self::Lua(e)
    }
}

impl From<crate::layout::LayoutError> for ScriptError {
    fn from(e: crate::layout::LayoutError) -> Self {
        Self::Layout(e)
    }
}
