use regex::Regex;

pub fn remove_fillers(text: &str, fillers: &[String]) -> String {
    let mut out = text.to_string();
    for f in fillers {
        // word-boundary, case-insensitive removal
        let pat = format!(r"(?i)\b{}\b[, ]*", regex::escape(f));
        if let Ok(re) = Regex::new(&pat) {
            out = re.replace_all(&out, "").to_string();
        }
    }
    // collapse multiple spaces
    let re = Regex::new(r"\s{2,}").unwrap();
    re.replace_all(&out, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removes_um() {
        let r = remove_fillers("um I think this is good", &["um".into()]);
        assert_eq!(r, "I think this is good");
    }
}
