pub fn is_executable_url(url: &str) -> bool {
    let Some((prefix, data)) = url.trim_start().split_once(':') else {
        return false;
    };
    let normalised_value = prefix.to_lowercase().replace(['\t', '\n', '\r'], "");

    if matches!(normalised_value.as_str(), "javascript" | "vbscript") {
        return true;
    }
    if normalised_value.as_str() == "data" {
        let Some((media_type, _)) = data.split_once([',', ';']) else {
            return false;
        };
        matches!(
            media_type.trim(),
            "application/xhtml+xml" | "image/svg+xml" | "text/html"
        )
    } else {
        false
    }
}
