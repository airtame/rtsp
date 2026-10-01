#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
#[non_exhaustive]
pub enum StatusCode {
    Continue = 100,
    Ok = 200,
    Created = 201,
    LowOnStorageSpace = 250,
    MultipleChoices = 300,
    MovedPermanently = 301,
    MovedTemporarily = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    RequestEntityTooLarge = 413,
    RequestUriTooLarge = 414,
    UnsupportedMediaType = 415,
    ParameterNotUnderstood = 451,
    ConferenceNotFound = 452,
    NotEnoughBandwidth = 453,
    SessionNotFound = 454,
    MethodNotValidInThisState = 455,
    HeaderFieldNotValidForResource = 456,
    InvalidRange = 457,
    ParameterIsReadOnly = 458,
    AggregateOperationNotAllowed = 459,
    OnlyAggregateOperationAllowed = 460,
    UnsupportedTransport = 461,
    DestinationUnreachable = 462,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    RtspVersionNotSupported = 505,
    OptionNotSupported = 551,
}

impl StatusCode {
    pub fn code(self) -> u16 {
        self as u16
    }

    pub fn reason_phrase(self) -> &'static str {
        match self {
            Self::Continue => "Continue",
            Self::Ok => "OK",
            Self::Created => "Created",
            Self::LowOnStorageSpace => "Low on Storage Space",
            Self::MultipleChoices => "Multiple Choices",
            Self::MovedPermanently => "Moved Permanently",
            Self::MovedTemporarily => "Moved Temporarily",
            Self::SeeOther => "See Other",
            Self::NotModified => "Not Modified",
            Self::UseProxy => "Use Proxy",
            Self::BadRequest => "Bad Request",
            Self::Unauthorized => "Unauthorized",
            Self::PaymentRequired => "Payment Required",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::NotAcceptable => "Not Acceptable",
            Self::ProxyAuthenticationRequired => "Proxy Authentication Required",
            Self::RequestTimeout => "Request Time-out",
            Self::Gone => "Gone",
            Self::LengthRequired => "Length Required",
            Self::PreconditionFailed => "Precondition Failed",
            Self::RequestEntityTooLarge => "Request Entity Too Large",
            Self::RequestUriTooLarge => "Request-URI Too Large",
            Self::UnsupportedMediaType => "Unsupported Media Type",
            Self::ParameterNotUnderstood => "Parameter Not Understood",
            Self::ConferenceNotFound => "Conference Not Found",
            Self::NotEnoughBandwidth => "Not Enough Bandwidth",
            Self::SessionNotFound => "Session Not Found",
            Self::MethodNotValidInThisState => "Method Not Valid in This State",
            Self::HeaderFieldNotValidForResource => "Header Field Not Valid for Resource",
            Self::InvalidRange => "Invalid Range",
            Self::ParameterIsReadOnly => "Parameter Is Read-Only",
            Self::AggregateOperationNotAllowed => "Aggregate operation not allowed",
            Self::OnlyAggregateOperationAllowed => "Only aggregate operation allowed",
            Self::UnsupportedTransport => "Unsupported transport",
            Self::DestinationUnreachable => "Destination unreachable",
            Self::InternalServerError => "Internal Server Error",
            Self::NotImplemented => "Not Implemented",
            Self::BadGateway => "Bad Gateway",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::GatewayTimeout => "Gateway Time-out",
            Self::RtspVersionNotSupported => "RTSP Version not supported",
            Self::OptionNotSupported => "Option not supported",
        }
    }
}

impl From<StatusCode> for u16 {
    fn from(status_code: StatusCode) -> Self {
        status_code.code()
    }
}

impl TryFrom<u16> for StatusCode {
    type Error = u16;

    fn try_from(code: u16) -> Result<Self, Self::Error> {
        Ok(match code {
            100 => Self::Continue,
            200 => Self::Ok,
            201 => Self::Created,
            250 => Self::LowOnStorageSpace,
            300 => Self::MultipleChoices,
            301 => Self::MovedPermanently,
            302 => Self::MovedTemporarily,
            303 => Self::SeeOther,
            304 => Self::NotModified,
            305 => Self::UseProxy,
            400 => Self::BadRequest,
            401 => Self::Unauthorized,
            402 => Self::PaymentRequired,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            405 => Self::MethodNotAllowed,
            406 => Self::NotAcceptable,
            407 => Self::ProxyAuthenticationRequired,
            408 => Self::RequestTimeout,
            410 => Self::Gone,
            411 => Self::LengthRequired,
            412 => Self::PreconditionFailed,
            413 => Self::RequestEntityTooLarge,
            414 => Self::RequestUriTooLarge,
            415 => Self::UnsupportedMediaType,
            451 => Self::ParameterNotUnderstood,
            452 => Self::ConferenceNotFound,
            453 => Self::NotEnoughBandwidth,
            454 => Self::SessionNotFound,
            455 => Self::MethodNotValidInThisState,
            456 => Self::HeaderFieldNotValidForResource,
            457 => Self::InvalidRange,
            458 => Self::ParameterIsReadOnly,
            459 => Self::AggregateOperationNotAllowed,
            460 => Self::OnlyAggregateOperationAllowed,
            461 => Self::UnsupportedTransport,
            462 => Self::DestinationUnreachable,
            500 => Self::InternalServerError,
            501 => Self::NotImplemented,
            502 => Self::BadGateway,
            503 => Self::ServiceUnavailable,
            504 => Self::GatewayTimeout,
            505 => Self::RtspVersionNotSupported,
            551 => Self::OptionNotSupported,
            _ => return Err(code),
        })
    }
}

#[cfg(test)]
#[path = "test/status_code_unittests.rs"]
mod tests;
