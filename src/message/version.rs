#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Version {
    V1,
    V2,
    Other(String),
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let token = match self {
            Self::V1 => "RTSP/1.0",
            Self::V2 => "RTSP/2.0",
            Self::Other(token) => token,
        };
        f.write_str(token)
    }
}

impl std::str::FromStr for Version {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let token = s.trim();

        match token.to_ascii_uppercase().as_str() {
            "RTSP/1.0" => Ok(Self::V1),
            "RTSP/2.0" => Ok(Self::V2),
            uppercase if !uppercase.starts_with("RTSP/") && is_version_token(token) => {
                Ok(Self::Other(token.to_owned()))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid version: {s:?}"),
            )),
        }
    }
}

fn is_version_token(token: &str) -> bool {
    let Some((name, number)) = token.split_once('/') else {
        return false;
    };
    let Some((major, minor)) = number.split_once('.') else {
        return false;
    };
    let is_number = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());

    !name.is_empty()
        && name.bytes().all(|byte| byte.is_ascii_alphabetic())
        && is_number(major)
        && is_number(minor)
}

#[cfg(test)]
#[path = "test/version_unittests.rs"]
mod tests;
