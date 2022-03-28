use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum ContentType {
    Css,
    Csv,
    EventStream,
    FormUrlEncoded,
    Gif,
    Html,
    JavaScript,
    Jpeg,
    Json,
    Markdown,
    MultipartForm,
    None,
    OctetStream,
    Pdf,
    PlainText,
    Png,
    Svg,
    Str(&'static str),
    String(String),
}
impl ContentType {
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.split(';').next() {
            Some("text/css") => ContentType::Css,
            Some("text/csv") => ContentType::Csv,
            Some("text/event-stream") => ContentType::EventStream,
            Some("application/x-www-form-urlencoded") => ContentType::FormUrlEncoded,
            Some("image/gif") => ContentType::Gif,
            Some("text/html") => ContentType::Html,
            Some("text/javascript") => ContentType::JavaScript,
            Some("image/jpeg") => ContentType::Jpeg,
            Some("application/json") => ContentType::Json,
            Some("text/markdown") => ContentType::Markdown,
            Some("multipart/form-data") => ContentType::MultipartForm,
            Some("") => ContentType::None,
            Some("application/octet-stream") => ContentType::OctetStream,
            Some("application/pdf") => ContentType::Pdf,
            Some("text/plain") => ContentType::PlainText,
            Some("image/png") => ContentType::Png,
            Some("image/svg+xml") => ContentType::Svg,
            _ => ContentType::String(s.to_string()),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            ContentType::Css => "text/css; charset=UTF-8",
            ContentType::Csv => "text/csv; charset=UTF-8",
            ContentType::EventStream => "text/event-stream",
            ContentType::FormUrlEncoded => "application/x-www-form-urlencoded; charset=UTF-8",
            ContentType::Gif => "image/gif",
            ContentType::Html => "text/html; charset=UTF-8",
            ContentType::JavaScript => "text/javascript; charset=UTF-8",
            ContentType::Jpeg => "image/jpeg",
            ContentType::Json => "application/json; charset=UTF-8",
            ContentType::Markdown => "text/markdown; charset=UTF-8",
            ContentType::MultipartForm => "multipart/form-data",
            ContentType::None => "",
            ContentType::OctetStream => "application/octet-stream",
            ContentType::Pdf => "application/pdf",
            ContentType::PlainText => "text/plain; charset=UTF-8",
            ContentType::Png => "image/png",
            ContentType::Svg => "image/svg+xml; charset=UTF-8",
            ContentType::Str(s) => s,
            ContentType::String(s) => s,
        }
    }
}
impl AsRef<str> for ContentType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl From<String> for ContentType {
    fn from(s: String) -> Self {
        Self::parse(&s)
    }
}
impl From<ContentType> for String {
    fn from(t: ContentType) -> Self {
        t.as_str().to_string()
    }
}
impl FromStr for ContentType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ContentType::parse(s))
    }
}
