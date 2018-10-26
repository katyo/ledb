use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// Generic string indentifier
///
/// This type is used as name for collections and fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Identifier {
    Owned(String),
    Refer(&'static str),
}

impl Default for Identifier {
    fn default() -> Self {
        Identifier::Refer("")
    }
}

impl Eq for Identifier {}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Hash for Identifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use self::Identifier::*;
        match self {
            Owned(s) => s.hash(state),
            Refer(s) => s.hash(state),
        }
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        use self::Identifier::*;
        match self {
            Owned(s) => &s,
            Refer(s) => s,
        }
    }
}

impl Borrow<str> for Identifier {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}

impl Deref for Identifier {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_ref()
    }
}

impl From<&'static str> for Identifier {
    fn from(s: &'static str) -> Self {
        Identifier::Refer(s)
    }
}

impl From<String> for Identifier {
    fn from(s: String) -> Self {
        Identifier::Owned(s)
    }
}

impl<'a> From<&'a String> for Identifier {
    fn from(s: &String) -> Self {
        Identifier::Owned(s.clone())
    }
}
