//! Defines a global symbol type and its associated interning pool
use std::{mem, sync};
use string_interner::{
    backend::BucketBackend, symbol::SymbolU32, StringInterner,
};

/// A Globally interned symbol.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize),
    serde(into = "&'static str")
)]
pub struct GSym(SymbolU32);

type Pool = StringInterner<BucketBackend>;

fn singleton() -> &'static mut Pool {
    static mut SINGLETON: mem::MaybeUninit<Pool> = mem::MaybeUninit::uninit();
    static ONCE: sync::Once = sync::Once::new();

    // SAFETY:
    // - writing to the singleton is OK because we only do it one time
    // - the ONCE guarantees that SINGLETON is init'ed before assume_init_ref
    unsafe {
        ONCE.call_once(|| {
            SINGLETON.write(Pool::new());
        });
        SINGLETON.assume_init_mut()
    }
}

impl GSym {
    /// Intern a string into the global symbol table.
    pub fn new(s: impl AsRef<str>) -> Self {
        s.as_ref().into()
    }

    /// Convert this symbol into the string in the static, global symbol table.
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }
}

impl From<&str> for GSym {
    fn from(s: &str) -> Self {
        GSym(singleton().get_or_intern(s))
    }
}

impl From<String> for GSym {
    fn from(s: String) -> Self {
        GSym(singleton().get_or_intern(&s))
    }
}

impl From<&String> for GSym {
    fn from(s: &String) -> Self {
        GSym(singleton().get_or_intern(s))
    }
}

impl From<GSym> for &'static str {
    fn from(sym: GSym) -> Self {
        singleton().resolve(sym.0).unwrap()
    }
}

impl std::fmt::Debug for GSym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for GSym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

#[cfg(feature = "serialize")]
struct StrVisitor;

#[cfg(feature = "serialize")]
impl<'de> serde::de::Visitor<'de> for StrVisitor {
    type Value = GSym;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        formatter.write_str("a &str")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for GSym {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let x = deserializer.deserialize_str(StrVisitor)?;
        Ok(x)
    }
}
