use crate::error::FilterError;
use regex::Regex;

#[derive(Debug, Clone)]
pub enum DomainPattern {
    Exact(String),
    Wildcard(String),
    Regex(Regex),
}

impl DomainPattern {
    pub fn new_exact(domain: &str) -> Self {
        Self::Exact(domain.to_lowercase())
    }

    pub fn new_wildcard(pattern: &str) -> Self {
        Self::Wildcard(pattern.to_lowercase())
    }

    pub fn new_regex(pattern: &str) -> Result<Self, FilterError> {
        let regex = Regex::new(pattern)?;
        Ok(Self::Regex(regex))
    }

    pub fn matches(&self, domain: &str) -> bool {
        let domain = domain.to_lowercase();
        match self {
            Self::Exact(p) => *p == domain,
            Self::Wildcard(p) => {
                if let Some(suffix) = p.strip_prefix("*.") {
                    domain.ends_with(suffix) && domain.len() > suffix.len() + 1
                } else {
                    *p == domain
                }
            }
            Self::Regex(r) => r.is_match(&domain),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let pattern = DomainPattern::new_exact("example.com");
        assert!(pattern.matches("example.com"));
        assert!(!pattern.matches("other.com"));
    }

    #[test]
    fn exact_case_insensitive() {
        let pattern = DomainPattern::new_exact("Example.COM");
        assert!(pattern.matches("example.com"));
        assert!(pattern.matches("EXAMPLE.COM"));
    }

    #[test]
    fn wildcard_match() {
        let pattern = DomainPattern::new_wildcard("*.example.com");
        assert!(pattern.matches("api.example.com"));
        assert!(pattern.matches("www.example.com"));
    }

    #[test]
    fn wildcard_no_match_bare_domain() {
        let pattern = DomainPattern::new_wildcard("*.example.com");
        assert!(!pattern.matches("example.com"));
    }

    #[test]
    fn wildcard_case_insensitive() {
        let pattern = DomainPattern::new_wildcard("*.Example.COM");
        assert!(pattern.matches("API.Example.Com"));
    }

    #[test]
    fn wildcard_no_prefix() {
        let pattern = DomainPattern::new_wildcard("example.com");
        assert!(pattern.matches("example.com"));
        assert!(!pattern.matches("api.example.com"));
    }

    #[test]
    fn regex_match() {
        let pattern = DomainPattern::new_regex(r"^api\..*").unwrap();
        assert!(pattern.matches("api.example.com"));
        assert!(pattern.matches("api.other.com"));
        assert!(!pattern.matches("www.example.com"));
    }

    #[test]
    fn regex_invalid() {
        let result = DomainPattern::new_regex("[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn empty_domain() {
        let pattern = DomainPattern::new_exact("example.com");
        assert!(!pattern.matches(""));
    }

    #[test]
    fn wildcard_deep_subdomain() {
        let pattern = DomainPattern::new_wildcard("*.example.com");
        assert!(pattern.matches("a.b.example.com"));
    }
}
