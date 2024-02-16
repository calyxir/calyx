use std::path::PathBuf;

use crate::{document::Document, Config};

pub trait QueryResult2: Sized + std::fmt::Debug {
    type Data;
    type Needle;

    fn found(&self) -> Option<Self::Data>;
    fn paths(&self) -> Vec<PathBuf>;
    fn resume(&self, config: &Config, doc: &Document) -> Option<Self>;

    fn resolve<F>(&self, f: F) -> Option<Self::Data>
    where
        F: Fn(&Self, &PathBuf) -> Option<Self> + Clone,
    {
        // if self.found() has data, then return that
        // else search for the first path where f returns something
        self.found().or_else(|| {
            self.paths()
                .iter()
                .find_map(|p| f(&self, &p).and_then(|res| res.resolve(f.clone())))
        })
    }
}
