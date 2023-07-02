use std::path::Path;

pub trait PathLike<'a> {
    fn path(&'a self) -> &'a Path;

    fn is_system(&'a self) -> bool {
        Self::path(self).starts_with("/usr")
    }
}

impl<'a> PathLike<'a> for std::path::PathBuf {
    fn path(&'a self) -> &'a Path {
        &self
    }
}
