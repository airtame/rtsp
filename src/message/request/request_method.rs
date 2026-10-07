#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RequestMethod {
    Describe,
    Announce,
    GetParameter,
    Options,
    Pause,
    Play,
    Record,
    Redirect,
    Setup,
    SetParameter,
    Teardown,
    Extension(String),
}

impl std::fmt::Display for RequestMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let token = match self {
            Self::Describe => "DESCRIBE",
            Self::Announce => "ANNOUNCE",
            Self::GetParameter => "GET_PARAMETER",
            Self::Options => "OPTIONS",
            Self::Pause => "PAUSE",
            Self::Play => "PLAY",
            Self::Record => "RECORD",
            Self::Redirect => "REDIRECT",
            Self::Setup => "SETUP",
            Self::SetParameter => "SET_PARAMETER",
            Self::Teardown => "TEARDOWN",
            Self::Extension(token) => token,
        };
        f.write_str(token)
    }
}

impl std::str::FromStr for RequestMethod {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let token = s.trim();

        Ok(match token.to_ascii_uppercase().as_str() {
            "DESCRIBE" => Self::Describe,
            "ANNOUNCE" => Self::Announce,
            "GET_PARAMETER" => Self::GetParameter,
            "OPTIONS" => Self::Options,
            "PAUSE" => Self::Pause,
            "PLAY" => Self::Play,
            "RECORD" => Self::Record,
            "REDIRECT" => Self::Redirect,
            "SETUP" => Self::Setup,
            "SET_PARAMETER" => Self::SetParameter,
            "TEARDOWN" => Self::Teardown,
            _ => Self::Extension(token.to_owned()),
        })
    }
}

#[cfg(test)]
#[path = "test/request_method_unittests.rs"]
mod tests;
