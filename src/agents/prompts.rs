//! Provider-specific prompt tuning.
//!
//! Returns a suffix appended to the system prompt based on the active
//! provider. Hardcoded per provider (versioned with code, not config).

/// Returns a provider-specific prompt suffix to append to the system prompt.
///
/// Different LLM providers benefit from different phrasing. For example,
/// some providers handle complex JSON tool calls better with explicit
/// reminders about format.
#[must_use]
pub fn provider_prompt_suffix(provider_name: &str) -> &'static str {
    match provider_name {
        "anthropic" => "",
        "openai" => {
            "\n\nReminder: emit tool calls using the function calling format. \
                      Each tool call's `arguments` must be a valid JSON string."
        }
        "grok" => {
            "\n\nReminder: respond ONLY with tool calls. Do not include explanatory \
                   text before or after the tool calls. Each tool call must have valid JSON arguments."
        }
        "openrouter" => {
            "\n\nReminder: emit tool calls using the function calling format. \
                         Each tool call's `arguments` must be a valid JSON string. \
                         Respond ONLY with tool calls for the requested mutation."
        }
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_has_no_suffix() {
        assert!(provider_prompt_suffix("anthropic").is_empty());
    }

    #[test]
    fn openai_has_suffix() {
        let suffix = provider_prompt_suffix("openai");
        assert!(suffix.contains("function calling"));
    }

    #[test]
    fn grok_has_suffix() {
        let suffix = provider_prompt_suffix("grok");
        assert!(suffix.contains("ONLY with tool calls"));
    }

    #[test]
    fn openrouter_has_suffix() {
        let suffix = provider_prompt_suffix("openrouter");
        assert!(suffix.contains("function calling"));
    }

    #[test]
    fn unknown_provider_has_no_suffix() {
        assert!(provider_prompt_suffix("unknown").is_empty());
    }
}
