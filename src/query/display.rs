//! Query answer display helpers.

/// Query answer split into optional model-emitted thinking and visible answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayAnswer {
    /// Text returned inside well-formed `<think>...</think>` blocks.
    pub thinking: Option<String>,
    /// Answer text with well-formed thinking blocks removed.
    pub answer: String,
}

/// Splits model-visible `<think>...</think>` text from the final answer.
///
/// Malformed thinking tags are treated as normal answer text so output is never
/// silently dropped.
#[must_use]
pub fn split_thinking_blocks(text: &str) -> DisplayAnswer {
    let mut remainder = text;
    let mut answer = String::new();
    let mut thinking_parts = Vec::new();

    loop {
        let Some(start) = remainder.find("<think>") else {
            answer.push_str(remainder);
            break;
        };
        let content_start = start + "<think>".len();
        let Some(relative_end) = remainder[content_start..].find("</think>") else {
            return DisplayAnswer {
                thinking: None,
                answer: text.trim().to_string(),
            };
        };
        let end = content_start + relative_end;

        answer.push_str(&remainder[..start]);
        let thinking = remainder[content_start..end].trim();
        if !thinking.is_empty() {
            thinking_parts.push(thinking.to_string());
        }
        remainder = &remainder[end + "</think>".len()..];
    }

    DisplayAnswer {
        thinking: (!thinking_parts.is_empty()).then(|| thinking_parts.join("\n\n")),
        answer: answer.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_valid_thinking_block() {
        let parsed = split_thinking_blocks("<think>check context</think>Hello.");

        assert_eq!(parsed.thinking.as_deref(), Some("check context"));
        assert_eq!(parsed.answer, "Hello.");
    }

    #[test]
    fn removes_thinking_tags_from_answer() {
        let parsed = split_thinking_blocks("Before <think>hidden</think> after");

        assert_eq!(parsed.thinking.as_deref(), Some("hidden"));
        assert_eq!(parsed.answer, "Before  after");
    }

    #[test]
    fn falls_back_without_thinking_block() {
        let parsed = split_thinking_blocks("Plain answer");

        assert_eq!(parsed.thinking, None);
        assert_eq!(parsed.answer, "Plain answer");
    }

    #[test]
    fn malformed_thinking_keeps_full_answer() {
        let parsed = split_thinking_blocks("<think>unfinished");

        assert_eq!(parsed.thinking, None);
        assert_eq!(parsed.answer, "<think>unfinished");
    }

    #[test]
    fn empty_thinking_does_not_drop_answer() {
        let parsed = split_thinking_blocks("<think>  </think>Final");

        assert_eq!(parsed.thinking, None);
        assert_eq!(parsed.answer, "Final");
    }
}
