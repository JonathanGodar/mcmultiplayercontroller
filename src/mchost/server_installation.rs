use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs::DirEntry;

#[derive(Debug)]
pub struct ServerInstallation {
    pub jar_path: PathBuf,
    pub id: ServerInstallationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInstallationId {
    r#type: ServerInstallationType,
    version: SemVer,
}

impl ServerInstallationId {
    pub fn new(r#type: ServerInstallationType, major: u64, minor: u64, patch: u64) -> Self {
        Self {
            r#type,
            version: SemVer::new(major, minor, patch),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ServerInstallationType {
    Paper,
    Spigot,
}

impl ServerInstallationType {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "paper" => Some(Self::Paper),
            "spigot" => Some(Self::Spigot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl SemVer {
    fn from_str(value: &str) -> Option<Self> {
        let mut segments = value.split(".");

        let major = segments.next()?.parse().ok()?;
        let minor = segments.next()?.parse().ok()?;
        let patch = segments.next()?.parse().ok()?;

        if segments.next().is_some() {
            return None;
        }

        Some(Self {
            major,
            minor,
            patch,
        })
    }

    fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl ServerInstallation {
    pub async fn new(value: DirEntry) -> Option<Self> {
        if !value.file_type().await.unwrap().is_file() {
            return None;
        }

        let file_name = value.file_name().to_str()?.to_owned();
        let (file_base, ext) = file_name.rsplit_once(".")?;

        if ext != "jar" {
            return None;
        }

        let (server_type, server_version) = file_base.split_once('-')?;

        // Remove build number
        let server_version = server_version.split('-').next()?;

        let server_type = ServerInstallationType::from_str(server_type)?;
        let server_version = SemVer::from_str(server_version)?;

        Some(Self {
            id: ServerInstallationId {
                r#type: server_type,
                version: server_version,
            },
            jar_path: value.path(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::SemVer;

    #[test]
    fn sem_ver_parsing() {
        let one_two_three = SemVer::from_str("1.2.3");
        assert!(matches!(
            one_two_three,
            Some(SemVer {
                major: 1,
                minor: 2,
                patch: 3
            })
        ));

        let invalid = SemVer::from_str("31.123");
        assert_eq!(invalid, None);

        let invalid = SemVer::from_str("31 123.3");
        assert_eq!(invalid, None);

        let invalid = SemVer::from_str("");
        assert_eq!(invalid, None);

        let invalid = SemVer::from_str("...");
        assert_eq!(invalid, None);

        let invalid = SemVer::from_str("..");
        assert_eq!(invalid, None);

        let invalid = SemVer::from_str("1.2.3.4");
        assert_eq!(invalid, None);

        let invalid = SemVer::from_str("dasflkj1.2.3");
        assert_eq!(invalid, None);

        let invalid = SemVer::from_str("1.2.3saef");
        assert_eq!(invalid, None);
    }

    #[test]
    fn sem_ver_ordering() {
        let one_one_one = SemVer::from_str("1.1.1").unwrap();
        let one_one_two = SemVer::from_str("1.1.2").unwrap();
        let two_one_two = SemVer::from_str("2.1.2").unwrap();
        let two_one_one = SemVer::from_str("2.1.1").unwrap();
        let one_three_one = SemVer::from_str("1.3.1").unwrap();

        assert!(one_one_one < one_one_two);
        assert!(one_one_two < one_three_one);
        assert!(one_three_one < two_one_one);
        assert!(two_one_one < two_one_two);
    }
}
