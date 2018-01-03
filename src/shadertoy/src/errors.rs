error_chain!{
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
        Json(::serde_json::error::Error);
        Reqwest(::reqwest::Error);
    }
}
