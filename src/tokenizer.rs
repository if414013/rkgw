// Token counting module
// Provides approximate token counting for Claude models
//
// Note: This is an approximate count, as the exact Claude tokenizer
// is not public. Uses character-based estimation with a correction factor.
//
// The correction coefficient CLAUDE_CORRECTION_FACTOR = 1.15 is based on
// empirical observations: Claude tokenizes text approximately 15%
// more than GPT-4 (cl100k_base).

use serde_json::Value;
use crate::models::anthropic::AnthropicTool;

/// Correction coefficient for Claude models
/// Claude tokenizes text approximately 15% more than GPT-4 (cl100k_base)
const CLAUDE_CORRECTION_FACTOR: f64 = 1.15;

/// Counts the approximate number of tokens in text.
///
/// Uses character-based estimation (~4 characters per token for English).
/// Applies Claude correction factor by default.
///
/// # Arguments
/// * `text` - Text to count tokens for
/// * `apply_claude_correction` - Whether to apply the Claude correction factor
///
/// # Returns
/// Approximate number of tokens
pub fn count_tokens(text: &str, apply_claude_correction: bool) -> i32 {
    if text.is_empty() {
        return 0;
    }

    // Rough estimate: ~4 characters per token for English,
    // ~2-3 characters for other languages (taking average ~3.5)
    let base_estimate = (text.len() / 4 + 1) as f64;

    if apply_claude_correction {
        (base_estimate * CLAUDE_CORRECTION_FACTOR) as i32
    } else {
        base_estimate as i32
    }
}

/// Counts tokens in a list of Anthropic messages.
///
/// Accounts for message structure:
/// - role: ~1 token
/// - content: text tokens
/// - Service tokens between messages: ~3-4 tokens
///
/// # Arguments
/// * `messages` - List of messages in Anthropic format
/// * `system` - Optional system prompt
/// * `tools` - Optional tools definition
///
/// # Returns
/// Approximate number of input tokens
pub fn count_anthropic_message_tokens(
    messages: &[crate::models::anthropic::AnthropicMessage],
    system: Option<&Value>,
    tools: Option<&Vec<AnthropicTool>>,
) -> i32 {
    if messages.is_empty() && system.is_none() && tools.is_none() {
        return 0;
    }

    let mut total_tokens = 0;

    // Count system prompt tokens
    if let Some(sys) = system {
        total_tokens += 4; // Service tokens
        match sys {
            Value::String(s) => {
                total_tokens += count_tokens(s, false);
            }
            Value::Array(arr) => {
                for item in arr {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        total_tokens += count_tokens(text, false);
                    }
                }
            }
            _ => {}
        }
    }

    // Count message tokens
    for message in messages {
        // Base tokens per message (role, delimiters)
        total_tokens += 4;

        // Role tokens
        total_tokens += count_tokens(&message.role, false);

        // Content tokens
        match &message.content {
            Value::String(s) => {
                total_tokens += count_tokens(s, false);
            }
            Value::Array(arr) => {
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        match obj.get("type").and_then(|t| t.as_str()) {
                            Some("text") => {
                                if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                                    total_tokens += count_tokens(text, false);
                                }
                            }
                            Some("image") | Some("image_url") => {
                                // Images take ~85-170 tokens depending on size
                                total_tokens += 100;
                            }
                            Some("tool_use") => {
                                total_tokens += 4; // Service tokens
                                if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                                    total_tokens += count_tokens(name, false);
                                }
                                if let Some(input) = obj.get("input") {
                                    let input_str = serde_json::to_string(input).unwrap_or_default();
                                    total_tokens += count_tokens(&input_str, false);
                                }
                            }
                            Some("tool_result") => {
                                total_tokens += 4; // Service tokens
                                if let Some(tool_use_id) = obj.get("tool_use_id").and_then(|id| id.as_str()) {
                                    total_tokens += count_tokens(tool_use_id, false);
                                }
                                if let Some(content) = obj.get("content") {
                                    match content {
                                        Value::String(s) => {
                                            total_tokens += count_tokens(s, false);
                                        }
                                        Value::Array(arr) => {
                                            for c in arr {
                                                if let Some(text) = c.get("text").and_then(|t| t.as_str()) {
                                                    total_tokens += count_tokens(text, false);
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Some("thinking") => {
                                if let Some(thinking) = obj.get("thinking").and_then(|t| t.as_str()) {
                                    total_tokens += count_tokens(thinking, false);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Count tools tokens
    if let Some(tools_list) = tools {
        for tool in tools_list {
            total_tokens += 4; // Service tokens

            total_tokens += count_tokens(&tool.name, false);
            if let Some(ref desc) = tool.description {
                total_tokens += count_tokens(desc, false);
            }
            let schema_str = serde_json::to_string(&tool.input_schema).unwrap_or_default();
            total_tokens += count_tokens(&schema_str, false);
        }
    }

    // Final service tokens
    total_tokens += 3;

    // Apply Claude correction to total count
    (total_tokens as f64 * CLAUDE_CORRECTION_FACTOR) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::anthropic::AnthropicMessage;
    use serde_json::json;

    #[test]
    fn test_count_tokens_empty() {
        assert_eq!(count_tokens("", true), 0);
        assert_eq!(count_tokens("", false), 0);
    }

    #[test]
    fn test_count_tokens_simple() {
        // "Hello world" = 11 chars -> ~3 tokens base -> ~3-4 with correction
        let tokens = count_tokens("Hello world", true);
        assert!(tokens > 0);
        assert!(tokens < 20); // Sanity check
    }

    #[test]
    fn test_count_tokens_without_correction() {
        // Use a longer string so the correction factor makes a visible difference
        let long_text = "This is a much longer text that should have enough tokens to show the difference between corrected and uncorrected counts.";
        let with_correction = count_tokens(long_text, true);
        let without_correction = count_tokens(long_text, false);
        assert!(with_correction >= without_correction);
        // For longer text, correction should be strictly greater
        assert!(with_correction > without_correction);
    }

    #[test]
    fn test_count_anthropic_message_tokens_empty() {
        let messages: Vec<AnthropicMessage> = vec![];
        assert_eq!(count_anthropic_message_tokens(&messages, None, None), 0);
    }

    #[test]
    fn test_count_anthropic_message_tokens_simple() {
        let messages = vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: json!("Hello, how are you?"),
            },
        ];
        let tokens = count_anthropic_message_tokens(&messages, None, None);
        assert!(tokens > 0);
    }

    #[test]
    fn test_count_anthropic_message_tokens_with_system() {
        let messages = vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: json!("Hello"),
            },
        ];
        let system = json!("You are a helpful assistant.");
        let tokens = count_anthropic_message_tokens(&messages, Some(&system), None);
        assert!(tokens > 0);
    }

    #[test]
    fn test_count_anthropic_message_tokens_multimodal() {
        let messages = vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: json!([
                    {"type": "text", "text": "What's in this image?"},
                    {"type": "image", "source": {"type": "base64", "data": "..."}}
                ]),
            },
        ];
        let tokens = count_anthropic_message_tokens(&messages, None, None);
        assert!(tokens >= 100); // At least image tokens
    }
}
