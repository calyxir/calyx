use crate::Primitive;
use calyx_utils::Id;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::path::PathBuf;

#[derive(Debug)]
/// Tracks the information for [Primitive]s defined in the program.
pub enum PrimitiveInfo {
    /// An extern block that defines multiple primitives
    Extern {
        path: PathBuf,
        primitives: LinkedHashMap<Id, Primitive>,
        is_source: bool,
    },
    /// An inline primitive
    Inline {
        primitive: Primitive,
        is_source: bool,
    },
}
impl PrimitiveInfo {
    pub fn ext(
        path: PathBuf,
        primitives: LinkedHashMap<Id, Primitive>,
    ) -> Self {
        PrimitiveInfo::Extern {
            path,
            primitives,
            is_source: false,
        }
    }

    pub fn inline(primitive: Primitive) -> Self {
        PrimitiveInfo::Inline {
            primitive,
            is_source: false,
        }
    }

    /// Check if this primitive is a source primitive
    pub fn is_source(&self) -> bool {
        match self {
            PrimitiveInfo::Extern { is_source, .. } => *is_source,
            PrimitiveInfo::Inline { is_source, .. } => *is_source,
        }
    }

    /// Mark this primitive as a source primitive
    pub fn set_source(&mut self) {
        match self {
            PrimitiveInfo::Extern {
                ref mut is_source, ..
            } => *is_source = true,
            PrimitiveInfo::Inline {
                ref mut is_source, ..
            } => *is_source = true,
        }
    }
}

/// A representation of all the primitive definitions found while parsing
/// the root program.
#[derive(Debug, Default)]
pub struct LibrarySignatures {
    /// The primitives defined in the current context.
    prims: Vec<PrimitiveInfo>,
}

impl LibrarySignatures {
    /// Add a new inline primitive to the context.
    /// Panics if a primitive with the same name is already defined.
    pub fn add_inline_primitive(
        &mut self,
        primitive: Primitive,
    ) -> &mut PrimitiveInfo {
        assert!(
            primitive.body.is_some(),
            "inline primitive must have a body"
        );
        let name = primitive.name;
        if self.find_primitive(name).is_some() {
            panic!("Primitive `{}` is already defined in the context.", name);
        }
        let prim = PrimitiveInfo::inline(primitive);
        self.prims.push(prim);
        self.prims.last_mut().unwrap()
    }

    /// Add a new, non-inline primitive to the context.
    /// Panics if a primitive with the same name is already defined.
    /// Requires that the file path is absolute and canonical.
    pub fn add_extern_primitive(
        &mut self,
        file: PathBuf,
        primitive: Primitive,
    ) {
        assert!(
            primitive.body.is_none(),
            "non-inline primitive must not have a body"
        );
        let name = primitive.name;
        if self.find_primitive(name).is_some() {
            panic!("Primitive `{}` is already defined in the context.", name);
        }
        let definined_ext = self.prims.iter_mut().find(|prim| match prim {
            PrimitiveInfo::Extern { path, .. } => path == &file,
            _ => false,
        });
        if let Some(PrimitiveInfo::Extern { primitives, .. }) = definined_ext {
            primitives.insert(name, primitive);
        } else {
            let mut primitives = LinkedHashMap::new();
            primitives.insert(name, primitive);
            self.prims.push(PrimitiveInfo::ext(file, primitives));
        }
    }

    pub(crate) fn add_extern(
        &mut self,
        file: PathBuf,
        prims: Vec<Primitive>,
    ) -> &mut PrimitiveInfo {
        let definined_ext = self.prims.iter().any(|prim| match prim {
            PrimitiveInfo::Extern { path, .. } => path == &file,
            _ => false,
        });
        if definined_ext {
            panic!(
                "Extern block with file `{}` is already defined in the context",
                file.display()
            );
        }

        let ext = PrimitiveInfo::ext(
            file,
            prims.into_iter().map(|p| (p.name, p)).collect(),
        );
        self.prims.push(ext);
        self.prims.last_mut().unwrap()
    }

    /// Return the [Primitive] associated with the given name if defined, otherwise return None.
    pub fn find_primitive<S>(&self, name: S) -> Option<&Primitive>
    where
        S: Into<Id>,
    {
        let key = name.into();
        self.prims.iter().find_map(|prim| match prim {
            PrimitiveInfo::Extern { primitives, .. } => primitives.get(&key),
            PrimitiveInfo::Inline { primitive, .. } => {
                if primitive.name == key {
                    Some(primitive)
                } else {
                    None
                }
            }
        })
    }

    /// Return the [Primitive] associated to this Id.
    pub fn get_primitive<S>(&self, name: S) -> &Primitive
    where
        S: Into<Id>,
    {
        let key = name.into();
        self.find_primitive(key).unwrap_or_else(|| {
            panic!("Primitive `{}` is not defined in the context.", key)
        })
    }

    /// Mark an inlined primitive as a part of the source.
    /// This is useful when using file mode compilation and printing only the source primitives.
    /// Panics if the primitive is not defined.
    pub fn mark_inline_source(&mut self, name: Id) {
        let Some(inlined) = self.prims.iter_mut().find(|prim| match prim {
            PrimitiveInfo::Inline { primitive, .. } => primitive.name == name,
            PrimitiveInfo::Extern { .. } => false,
        }) else {
            panic!("Primitive `{}` is not defined in the context.", name);
        };
        inlined.set_source()
    }

    /// Marks an `import`ed extern block as a part of the source.
    /// There is no way to mark an individual primitive as a part of the source since the entire file will be linked.
    /// Panics if the file path is not defined
    pub fn mark_extern_source(&mut self, path: PathBuf) {
        let Some(ext_def) = self.prims.iter_mut().find(|prim| match prim {
            PrimitiveInfo::Extern { path: p, .. } => p == &path,
            PrimitiveInfo::Inline { .. } => false,
        }) else {
            panic!(
                "extern file `{}` is not defined in the context",
                path.to_string_lossy()
            );
        };
        ext_def.set_source()
    }

    /// Return an iterator over all defined primitives.
    pub fn signatures(&self) -> impl Iterator<Item = &Primitive> + '_ {
        self.prims.iter().flat_map(|prim| match prim {
            PrimitiveInfo::Extern { primitives, .. } => {
                primitives.values().collect_vec()
            }
            PrimitiveInfo::Inline { primitive, .. } => vec![primitive],
        })
    }

    /// Returns all the underlying primitive information.
    /// If you want all the signatures, use [LibrarySignatures::signatures] instead.
    pub fn prim_infos(&self) -> &Vec<PrimitiveInfo> {
        &self.prims
    }

    /// Return the underyling inlined primitives and whether they are source defined
    pub fn prim_inlines(
        &self,
    ) -> impl Iterator<Item = (&Primitive, bool)> + '_ {
        self.prims.iter().flat_map(|prim| match prim {
            PrimitiveInfo::Extern { .. } => None,
            PrimitiveInfo::Inline {
                primitive,
                is_source,
            } => Some((primitive, *is_source)),
        })
    }

    /// Return the paths for the extern defining files along with whether they are source defined.
    pub fn extern_paths(&self) -> Vec<&PathBuf> {
        self.prims
            .iter()
            .filter_map(|p| match p {
                PrimitiveInfo::Extern { path, .. } => Some(path),
                PrimitiveInfo::Inline { .. } => None,
            })
            .collect_vec()
    }
}
