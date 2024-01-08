use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalizedPath(PathBuf);

impl AsRef<Path> for CanonicalizedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl TryFrom<lsp_types::Url> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::Url) -> Result<Self, Self::Error> {
        value
            .to_file_path()
            .map_err(|err| anyhow::anyhow!("{:?}", err))?
            .try_into()
    }
}

impl TryFrom<String> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        PathBuf::from(value).try_into()
    }
}

impl TryFrom<&Path> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        PathBuf::from(value).try_into()
    }
}

impl TryFrom<PathBuf> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Ok(Self(value.canonicalize().map_err(|error| {
            anyhow::anyhow!("Cannot canonicalize path: {:?}. Error: {:?}", value, error)
        })?))
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
        PathBuf::from(value).try_into()
    }
}

impl TryFrom<&String> for CanonicalizedPath {
    type Error = anyhow::Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        PathBuf::from(value).try_into()
    }
}

impl CanonicalizedPath {
    pub fn icon(&self) -> &String {
        let config = crate::icons::get_icon_config();
        self.0
            .file_name()
            .and_then(|filename| {
                config
                    .file_names
                    .get(&filename.to_string_lossy().to_string())
            })
            .or_else(|| config.file_extensions.get(self.extension()?))
            .unwrap_or(&config.file)
    }
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
        self.display_relative_to(&current_dir.try_into()?)
    }

    pub fn display_relative_to(&self, other: &CanonicalizedPath) -> anyhow::Result<String> {
        let relative = self.0.strip_prefix(&other.0)?;
        Ok(relative.display().to_string())
    }

    pub fn display_absolute(&self) -> String {
        self.0.display().to_string()
    }

    pub fn join(&self, other_path: &str) -> anyhow::Result<CanonicalizedPath> {
        let CanonicalizedPath(path) = self.clone();
        path.join(other_path).try_into()
    }

    pub fn remove_dir_all(&self) -> anyhow::Result<()> {
        Ok(std::fs::remove_dir_all(&self.0)?)
    }

    pub fn components(&self) -> Vec<String> {
        self.0
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
    }

    pub fn parent(&self) -> anyhow::Result<Option<CanonicalizedPath>> {
        self.0.parent().map(|path| path.try_into()).transpose()
    }

    pub fn is_dir(&self) -> bool {
        self.0.is_dir()
    }

    pub fn to_path_buf(&self) -> &PathBuf {
        &self.0
    }

    pub fn is_file(&self) -> bool {
        self.0.is_file()
    }

    pub fn join_as_path_buf(&self, other: &str) -> String {
        self.to_path_buf().join(other).to_string_lossy().to_string()
    }

    pub fn try_display_relative(&self) -> String {
        self.display_relative()
            .unwrap_or_else(|_| self.display_absolute())
    }
}
