use std::path::PathBuf;

use crate::{document::Document, Config};

/// Describes the result of document queries that potentially need to access other
/// documents. For example, to jump to definitions of imported primitives, we need
/// to walk imports searching for the relevent definition.
///
/// When a `QueryResult` is constructed, we don't have the locks for the documents
/// that we need to search, so we have to request them. We do this by returning the
/// paths of documents that we would like to search. The consumer of `QueryResult`
/// is responsible for searching through the returned paths, until
/// `QueryResult::found` returns `Some(..)`, or we run out of paths to search.
pub trait QueryResult: Sized + std::fmt::Debug {
    type Data;
    type Needle;

    /// Returns `Some(..)` if `self` represents having found whatever
    /// we are looking for
    fn found(&self) -> Option<Self::Data>;

    /// Return a list of paths to continue searching.
    fn paths(&self) -> Vec<PathBuf>;

    /// Search `doc` for whatever we are looking for.
    fn resume(&self, config: &Config, doc: &Document) -> Option<Self>;

    /// Resolve a `QueryResult` into `Self::Data` by walking over
    /// all returned paths, until `Self::found(..)` returns `Some`.
    fn resolve<F>(&self, f: F) -> Option<Self::Data>
    where
        F: Fn(&Self, &PathBuf) -> Option<Self> + Clone,
    {
        // if self.found() has data, then return that
        // else search for the first path where f returns something
        self.found().or_else(|| {
            self.paths()
                .iter()
                .find_map(|p| f(self, p).and_then(|res| res.resolve(f.clone())))
        })
    }
}
