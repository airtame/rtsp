#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MessageHeaderName {
    Accept,
    AcceptCredentials,
    AcceptEncoding,
    AcceptLanguage,
    AcceptRanges,
    Allow,
    AuthenticationInfo,
    Authorization,
    Bandwidth,
    Blocksize,
    CacheControl,
    Conference,
    Connection,
    ConnectionCredentials,
    ContentBase,
    ContentEncoding,
    ContentLanguage,
    ContentLength,
    ContentLocation,
    ContentType,
    CSeq,
    Date,
    Expires,
    From,
    Host,
    IfMatch,
    IfModifiedSince,
    IfNoneMatch,
    LastModified,
    Location,
    MediaProperties,
    MediaRange,
    MTag,
    NotifyReason,
    PipelinedRequests,
    ProxyAuthenticate,
    ProxyAuthenticationInfo,
    ProxyAuthorization,
    ProxyRequire,
    ProxySupported,
    Public,
    Range,
    Referer,
    Referrer,
    RequestStatus,
    Require,
    RetryAfter,
    RtpInfo,
    Scale,
    SeekStyle,
    Server,
    Session,
    Speed,
    Supported,
    TerminateReason,
    Timestamp,
    Transport,
    Unsupported,
    UserAgent,
    Vary,
    Via,
    WwwAuthenticate,
    Extension(String),
}

impl MessageHeaderName {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Accept => "Accept",
            Self::AcceptCredentials => "Accept-Credentials",
            Self::AcceptEncoding => "Accept-Encoding",
            Self::AcceptLanguage => "Accept-Language",
            Self::AcceptRanges => "Accept-Ranges",
            Self::Allow => "Allow",
            Self::AuthenticationInfo => "Authentication-Info",
            Self::Authorization => "Authorization",
            Self::Bandwidth => "Bandwidth",
            Self::Blocksize => "Blocksize",
            Self::CacheControl => "Cache-Control",
            Self::Conference => "Conference",
            Self::Connection => "Connection",
            Self::ConnectionCredentials => "Connection-Credentials",
            Self::ContentBase => "Content-Base",
            Self::ContentEncoding => "Content-Encoding",
            Self::ContentLanguage => "Content-Language",
            Self::ContentLength => "Content-Length",
            Self::ContentLocation => "Content-Location",
            Self::ContentType => "Content-Type",
            Self::CSeq => "CSeq",
            Self::Date => "Date",
            Self::Expires => "Expires",
            Self::From => "From",
            Self::Host => "Host",
            Self::IfMatch => "If-Match",
            Self::IfModifiedSince => "If-Modified-Since",
            Self::IfNoneMatch => "If-None-Match",
            Self::LastModified => "Last-Modified",
            Self::Location => "Location",
            Self::MediaProperties => "Media-Properties",
            Self::MediaRange => "Media-Range",
            Self::MTag => "MTag",
            Self::NotifyReason => "Notify-Reason",
            Self::PipelinedRequests => "Pipelined-Requests",
            Self::ProxyAuthenticate => "Proxy-Authenticate",
            Self::ProxyAuthenticationInfo => "Proxy-Authentication-Info",
            Self::ProxyAuthorization => "Proxy-Authorization",
            Self::ProxyRequire => "Proxy-Require",
            Self::ProxySupported => "Proxy-Supported",
            Self::Public => "Public",
            Self::Range => "Range",
            Self::Referer => "Referer",
            Self::Referrer => "Referrer",
            Self::RequestStatus => "Request-Status",
            Self::Require => "Require",
            Self::RetryAfter => "Retry-After",
            Self::RtpInfo => "RTP-Info",
            Self::Scale => "Scale",
            Self::SeekStyle => "Seek-Style",
            Self::Server => "Server",
            Self::Session => "Session",
            Self::Speed => "Speed",
            Self::Supported => "Supported",
            Self::TerminateReason => "Terminate-Reason",
            Self::Timestamp => "Timestamp",
            Self::Transport => "Transport",
            Self::Unsupported => "Unsupported",
            Self::UserAgent => "User-Agent",
            Self::Vary => "Vary",
            Self::Via => "Via",
            Self::WwwAuthenticate => "WWW-Authenticate",
            Self::Extension(name) => name,
        }
    }

    pub fn allows_multiple(&self) -> bool {
        matches!(
            self,
            Self::Accept
                | Self::AcceptEncoding
                | Self::AcceptLanguage
                | Self::AcceptRanges
                | Self::Allow
                | Self::AuthenticationInfo
                | Self::CacheControl
                | Self::Connection
                | Self::ContentEncoding
                | Self::ContentLanguage
                | Self::IfMatch
                | Self::IfNoneMatch
                | Self::MediaProperties
                | Self::MediaRange
                | Self::ProxyAuthenticate
                | Self::ProxyAuthenticationInfo
                | Self::ProxyRequire
                | Self::ProxySupported
                | Self::Public
                | Self::Range
                | Self::Require
                | Self::RtpInfo
                | Self::Supported
                | Self::Transport
                | Self::Unsupported
                | Self::Vary
                | Self::Via
                | Self::WwwAuthenticate
                | Self::Extension(_)
        )
    }
}

impl std::fmt::Display for MessageHeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq for MessageHeaderName {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq_ignore_ascii_case(other.as_str())
    }
}

impl Eq for MessageHeaderName {}

impl std::hash::Hash for MessageHeaderName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for byte in self.as_str().bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
        state.write_u8(0xff);
    }
}

impl std::str::FromStr for MessageHeaderName {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl From<&str> for MessageHeaderName {
    fn from(name: &str) -> Self {
        let name = name.trim();

        match name.to_ascii_lowercase().as_str() {
            "accept" => Self::Accept,
            "accept-credentials" => Self::AcceptCredentials,
            "accept-encoding" => Self::AcceptEncoding,
            "accept-language" => Self::AcceptLanguage,
            "accept-ranges" => Self::AcceptRanges,
            "allow" => Self::Allow,
            "authentication-info" => Self::AuthenticationInfo,
            "authorization" => Self::Authorization,
            "bandwidth" => Self::Bandwidth,
            "blocksize" => Self::Blocksize,
            "cache-control" => Self::CacheControl,
            "conference" => Self::Conference,
            "connection" => Self::Connection,
            "connection-credentials" => Self::ConnectionCredentials,
            "content-base" => Self::ContentBase,
            "content-encoding" => Self::ContentEncoding,
            "content-language" => Self::ContentLanguage,
            "content-length" => Self::ContentLength,
            "content-location" => Self::ContentLocation,
            "content-type" => Self::ContentType,
            "cseq" => Self::CSeq,
            "date" => Self::Date,
            "expires" => Self::Expires,
            "from" => Self::From,
            "host" => Self::Host,
            "if-match" => Self::IfMatch,
            "if-modified-since" => Self::IfModifiedSince,
            "if-none-match" => Self::IfNoneMatch,
            "last-modified" => Self::LastModified,
            "location" => Self::Location,
            "media-properties" => Self::MediaProperties,
            "media-range" => Self::MediaRange,
            "mtag" => Self::MTag,
            "notify-reason" => Self::NotifyReason,
            "pipelined-requests" => Self::PipelinedRequests,
            "proxy-authenticate" => Self::ProxyAuthenticate,
            "proxy-authentication-info" => Self::ProxyAuthenticationInfo,
            "proxy-authorization" => Self::ProxyAuthorization,
            "proxy-require" => Self::ProxyRequire,
            "proxy-supported" => Self::ProxySupported,
            "public" => Self::Public,
            "range" => Self::Range,
            "referer" => Self::Referer,
            "referrer" => Self::Referrer,
            "request-status" => Self::RequestStatus,
            "require" => Self::Require,
            "retry-after" => Self::RetryAfter,
            "rtp-info" => Self::RtpInfo,
            "scale" => Self::Scale,
            "seek-style" => Self::SeekStyle,
            "server" => Self::Server,
            "session" => Self::Session,
            "speed" => Self::Speed,
            "supported" => Self::Supported,
            "terminate-reason" => Self::TerminateReason,
            "timestamp" => Self::Timestamp,
            "transport" => Self::Transport,
            "unsupported" => Self::Unsupported,
            "user-agent" => Self::UserAgent,
            "vary" => Self::Vary,
            "via" => Self::Via,
            "www-authenticate" => Self::WwwAuthenticate,
            _ => Self::Extension(name.to_owned()),
        }
    }
}

impl From<String> for MessageHeaderName {
    fn from(name: String) -> Self {
        Self::from(name.as_str())
    }
}

#[cfg(test)]
#[path = "test/message_header_name_unittests.rs"]
mod tests;
