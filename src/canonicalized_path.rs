use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalizedPath(PathBuf);

impl TryFrom<PathBuf> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Ok(Self(value.canonicalize()?))
    }
}

impl From<CanonicalizedPath> for PathBuf {
    fn from(val: CanonicalizedPath) -> Self {
        val.0
    }
}

impl TryFrom<&str> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(PathBuf::from(value).canonicalize()?))
    }
}

impl TryFrom<&String> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Ok(Self(PathBuf::from(value).canonicalize()?))
    }
}

impl CanonicalizedPath {
    pub fn read(&self) -> anyhow::Result<String> {
        Ok(std::fs::read_to_string(&self.0)?)
    }

    pub fn write(&self, content: &str) -> anyhow::Result<()> {
        Ok(std::fs::write(&self.0, content)?)
    }

    pub fn extension(&self) -> Option<&str> {
        self.0.extension().and_then(|s| s.to_str())
    }

    /// Get the relative path of this file from the current working directory.
    pub fn display_relative(&self) -> anyhow::Result<String> {
        let current_dir = std::env::current_dir()?;
        let relative = self.0.strip_prefix(current_dir)?;
        Ok(relative.display().to_string())
    }

    pub fn display(&self) -> String {
        self.0.display().to_string()
    }
}
