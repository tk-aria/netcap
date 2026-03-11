pub mod pattern;

use pattern::DomainPattern;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum CaptureDecision {
    Capture(Uuid),
    Passthrough,
    Default,
}

#[derive(Debug, Clone)]
pub enum FilterType {
    Include,
    Exclude,
}

#[derive(Debug, Clone)]
pub struct FilterRule {
    pub id: Uuid,
    pub name: String,
    pub filter_type: FilterType,
    pub pattern: DomainPattern,
    pub priority: i32,
    pub enabled: bool,
}

pub trait DomainMatcher: Send + Sync + 'static {
    fn evaluate(&self, domain: &str) -> CaptureDecision;
    fn add_rule(&mut self, rule: FilterRule);
    fn remove_rule(&mut self, id: &Uuid) -> bool;
    fn clear(&mut self);
    fn rules(&self) -> &[FilterRule];
}

pub struct DomainFilter {
    rules: Vec<FilterRule>,
}

impl DomainFilter {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }
}

impl Default for DomainFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainMatcher for DomainFilter {
    fn evaluate(&self, domain: &str) -> CaptureDecision {
        let mut sorted: Vec<&FilterRule> = self.rules.iter().filter(|r| r.enabled).collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted {
            if rule.pattern.matches(domain) {
                return match rule.filter_type {
                    FilterType::Include => CaptureDecision::Capture(rule.id),
                    FilterType::Exclude => CaptureDecision::Passthrough,
                };
            }
        }
        CaptureDecision::Default
    }

    fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }

    fn remove_rule(&mut self, id: &Uuid) -> bool {
        let len = self.rules.len();
        self.rules.retain(|r| r.id != *id);
        self.rules.len() < len
    }

    fn clear(&mut self) {
        self.rules.clear();
    }

    fn rules(&self) -> &[FilterRule] {
        &self.rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn include_rule(name: &str, pattern: DomainPattern, priority: i32) -> FilterRule {
        FilterRule {
            id: Uuid::now_v7(),
            name: name.to_string(),
            filter_type: FilterType::Include,
            pattern,
            priority,
            enabled: true,
        }
    }

    fn exclude_rule(name: &str, pattern: DomainPattern, priority: i32) -> FilterRule {
        FilterRule {
            id: Uuid::now_v7(),
            name: name.to_string(),
            filter_type: FilterType::Exclude,
            pattern,
            priority,
            enabled: true,
        }
    }

    #[test]
    fn no_rules_returns_default() {
        let filter = DomainFilter::new();
        matches!(filter.evaluate("example.com"), CaptureDecision::Default);
    }

    #[test]
    fn include_rule_captures() {
        let mut filter = DomainFilter::new();
        filter.add_rule(include_rule(
            "test",
            DomainPattern::new_exact("example.com"),
            0,
        ));
        matches!(
            filter.evaluate("example.com"),
            CaptureDecision::Capture(_)
        );
    }

    #[test]
    fn exclude_rule_passthroughs() {
        let mut filter = DomainFilter::new();
        filter.add_rule(exclude_rule(
            "test",
            DomainPattern::new_exact("example.com"),
            0,
        ));
        matches!(
            filter.evaluate("example.com"),
            CaptureDecision::Passthrough
        );
    }

    #[test]
    fn priority_ordering() {
        let mut filter = DomainFilter::new();
        filter.add_rule(include_rule(
            "low",
            DomainPattern::new_exact("example.com"),
            1,
        ));
        filter.add_rule(exclude_rule(
            "high",
            DomainPattern::new_exact("example.com"),
            10,
        ));
        matches!(
            filter.evaluate("example.com"),
            CaptureDecision::Passthrough
        );
    }

    #[test]
    fn remove_rule() {
        let mut filter = DomainFilter::new();
        let rule = include_rule("test", DomainPattern::new_exact("example.com"), 0);
        let id = rule.id;
        filter.add_rule(rule);
        assert_eq!(filter.rules().len(), 1);
        assert!(filter.remove_rule(&id));
        assert_eq!(filter.rules().len(), 0);
    }

    #[test]
    fn remove_nonexistent_rule() {
        let mut filter = DomainFilter::new();
        assert!(!filter.remove_rule(&Uuid::now_v7()));
    }

    #[test]
    fn clear_rules() {
        let mut filter = DomainFilter::new();
        filter.add_rule(include_rule(
            "a",
            DomainPattern::new_exact("a.com"),
            0,
        ));
        filter.add_rule(include_rule(
            "b",
            DomainPattern::new_exact("b.com"),
            0,
        ));
        assert_eq!(filter.rules().len(), 2);
        filter.clear();
        assert_eq!(filter.rules().len(), 0);
    }

    #[test]
    fn disabled_rule_ignored() {
        let mut filter = DomainFilter::new();
        let mut rule = include_rule("test", DomainPattern::new_exact("example.com"), 0);
        rule.enabled = false;
        filter.add_rule(rule);
        matches!(filter.evaluate("example.com"), CaptureDecision::Default);
    }

    #[test]
    fn unmatched_domain_returns_default() {
        let mut filter = DomainFilter::new();
        filter.add_rule(include_rule(
            "test",
            DomainPattern::new_exact("example.com"),
            0,
        ));
        matches!(filter.evaluate("other.com"), CaptureDecision::Default);
    }
}
