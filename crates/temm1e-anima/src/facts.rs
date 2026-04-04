//! Per-message raw fact extraction.
//!
//! Pure code — no LLM calls. Extracts `MessageFacts` and `InteractionFacts`
//! from raw text and interaction metadata using string operations only.

use crate::types::{InteractionFacts, MessageFacts};

/// Extract all message-level facts from raw text.
pub fn collect_message_facts(text: &str) -> MessageFacts {
    let char_count = text.len();
    let word_count = text.split_whitespace().count();

    let sentence_count = text
        .chars()
        .filter(|c| *c == '.' || *c == '!' || *c == '?')
        .count();
    let question_count = text.chars().filter(|c| *c == '?').count();
    let exclamation_count = text.chars().filter(|c| *c == '!').count();

    // Count emoji: characters in common emoji ranges (Emoticons, Dingbats, Symbols, etc.)
    let emoji_count = text
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            // Basic emoji ranges
            (0x1F600..=0x1F64F).contains(&cp)  // Emoticons
                || (0x1F300..=0x1F5FF).contains(&cp)  // Misc Symbols and Pictographs
                || (0x1F680..=0x1F6FF).contains(&cp)  // Transport and Map
                || (0x1F1E0..=0x1F1FF).contains(&cp)  // Flags
                || (0x2600..=0x26FF).contains(&cp)     // Misc symbols
                || (0x2700..=0x27BF).contains(&cp)     // Dingbats
                || (0xFE00..=0xFE0F).contains(&cp)     // Variation Selectors
                || (0x1F900..=0x1F9FF).contains(&cp)   // Supplemental Symbols
                || (0x1FA00..=0x1FA6F).contains(&cp)   // Chess Symbols
                || (0x1FA70..=0x1FAFF).contains(&cp)   // Symbols Extended-A
                || (0x200D..=0x200D).contains(&cp) // Zero Width Joiner
        })
        .count();

    // Count code blocks (``` delimiters)
    let code_block_count = text.matches("```").count();

    // Uppercase ratio: uppercase alpha chars / total alpha chars
    let alpha_chars = text.chars().filter(|c| c.is_alphabetic()).count();
    let upper_chars = text.chars().filter(|c| c.is_uppercase()).count();
    let uppercase_ratio = if alpha_chars > 0 {
        upper_chars as f32 / alpha_chars as f32
    } else {
        0.0
    };

    // Punctuation density: punctuation chars / total chars
    let punct_chars = text.chars().filter(|c| c.is_ascii_punctuation()).count();
    let punctuation_density = if char_count > 0 {
        punct_chars as f32 / char_count as f32
    } else {
        0.0
    };

    // Average sentence length in words
    let avg_sentence_length = if sentence_count > 0 {
        word_count as f32 / sentence_count as f32
    } else {
        word_count as f32
    };

    // Language detection placeholder
    let language_detected = "unknown".to_string();

    let lower = text.to_lowercase();

    // Greeting detection
    let contains_greeting = [
        "hello",
        "hi ",
        "hi!",
        "hi.",
        "hey",
        "good morning",
        "good evening",
    ]
    .iter()
    .any(|g| lower.contains(g))
        || lower == "hi";

    // Thanks detection
    let contains_thanks = ["thank", "thanks", "thx", "ty "]
        .iter()
        .any(|t| lower.contains(t))
        || lower.ends_with("ty");

    // Apology detection
    let contains_apology = lower.contains("sorry") || lower.contains("apolog");

    // Question detection
    let contains_question = question_count > 0;

    // Command detection: rough heuristic — starts with a lowercase verb-like word
    let contains_command = detect_command_pattern(text);

    MessageFacts {
        char_count,
        word_count,
        sentence_count,
        question_count,
        exclamation_count,
        emoji_count,
        code_block_count,
        uppercase_ratio,
        punctuation_density,
        avg_sentence_length,
        language_detected,
        contains_greeting,
        contains_thanks,
        contains_apology,
        contains_question,
        contains_command,
    }
}

/// Detect if the text starts with a command-like pattern.
///
/// Heuristic: first word is a common imperative verb, or the sentence starts
/// with a lowercase word followed by a noun-like word (rough approximation).
fn detect_command_pattern(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    let first_word = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();

    // Strip trailing punctuation for comparison
    let first_word_clean: String = first_word
        .chars()
        .take_while(|c| c.is_alphabetic())
        .collect();

    // Common imperative verbs
    let imperative_verbs = [
        "run",
        "start",
        "stop",
        "build",
        "deploy",
        "create",
        "delete",
        "remove",
        "install",
        "update",
        "show",
        "list",
        "find",
        "search",
        "fix",
        "check",
        "test",
        "send",
        "open",
        "close",
        "write",
        "read",
        "make",
        "set",
        "get",
        "add",
        "edit",
        "move",
        "copy",
        "help",
        "explain",
        "tell",
        "give",
        "do",
        "execute",
        "fetch",
        "download",
        "upload",
        "compile",
        "debug",
        "restart",
        "kill",
        "push",
        "pull",
        "commit",
        "merge",
        "rebase",
        "configure",
    ];

    imperative_verbs.contains(&first_word_clean.as_str())
}

/// Collect interaction-level facts for a single turn.
pub fn collect_interaction_facts(
    seconds_since_last: u64,
    session_turn: u32,
    task_completed: bool,
    task_failed: bool,
    tool_calls: u32,
) -> InteractionFacts {
    InteractionFacts {
        seconds_since_last_message: seconds_since_last,
        session_turn_number: session_turn,
        topic_shifted: false, // Placeholder — topic shift detection requires LLM
        task_completed,
        task_failed,
        tool_calls_count: tool_calls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        let facts = collect_message_facts("");
        assert_eq!(facts.char_count, 0);
        assert_eq!(facts.word_count, 0);
        assert_eq!(facts.sentence_count, 0);
        assert_eq!(facts.question_count, 0);
        assert_eq!(facts.emoji_count, 0);
        assert!((facts.uppercase_ratio - 0.0).abs() < f32::EPSILON);
        assert!((facts.punctuation_density - 0.0).abs() < f32::EPSILON);
        assert!(!facts.contains_greeting);
        assert!(!facts.contains_thanks);
        assert!(!facts.contains_apology);
        assert!(!facts.contains_question);
        assert!(!facts.contains_command);
        assert_eq!(facts.language_detected, "unknown");
    }

    #[test]
    fn simple_sentence() {
        let facts = collect_message_facts("Hello world.");
        assert_eq!(facts.char_count, 12);
        assert_eq!(facts.word_count, 2);
        assert_eq!(facts.sentence_count, 1);
        assert_eq!(facts.question_count, 0);
        assert!(facts.contains_greeting);
        assert!(!facts.contains_question);
    }

    #[test]
    fn multiple_sentences_mixed_punctuation() {
        let facts = collect_message_facts("How are you? I'm fine! Great.");
        assert_eq!(facts.sentence_count, 3); // ? ! .
        assert_eq!(facts.question_count, 1);
        assert_eq!(facts.exclamation_count, 1);
        assert!(facts.contains_question);
        // avg sentence length: 6 words / 3 sentences = 2.0
        assert!((facts.avg_sentence_length - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn text_with_code_blocks() {
        let text = "Here is some code:\n```rust\nfn main() {}\n```\nDone.";
        let facts = collect_message_facts(text);
        assert_eq!(facts.code_block_count, 2); // Opening and closing ```
    }

    #[test]
    fn text_with_greeting() {
        assert!(collect_message_facts("hello there").contains_greeting);
        assert!(collect_message_facts("Hey!").contains_greeting);
        assert!(collect_message_facts("Good morning everyone").contains_greeting);
        assert!(collect_message_facts("hi").contains_greeting);
        assert!(!collect_message_facts("thinking about it").contains_greeting);
    }

    #[test]
    fn text_with_thanks() {
        assert!(collect_message_facts("thank you so much").contains_thanks);
        assert!(collect_message_facts("thanks!").contains_thanks);
        assert!(collect_message_facts("thx").contains_thanks);
        assert!(collect_message_facts("ty").contains_thanks);
        assert!(!collect_message_facts("typing fast").contains_thanks);
    }

    #[test]
    fn text_with_apology() {
        assert!(collect_message_facts("sorry about that").contains_apology);
        assert!(collect_message_facts("I apologize").contains_apology);
        assert!(!collect_message_facts("I am happy").contains_apology);
    }

    #[test]
    fn text_with_questions() {
        let facts = collect_message_facts("What? Why? How?");
        assert_eq!(facts.question_count, 3);
        assert!(facts.contains_question);
    }

    #[test]
    fn text_with_commands() {
        assert!(collect_message_facts("run the tests").contains_command);
        assert!(collect_message_facts("deploy to production").contains_command);
        assert!(collect_message_facts("show me the logs").contains_command);
        assert!(collect_message_facts("fix the bug").contains_command);
    }

    #[test]
    fn uppercase_ratio_calculation() {
        let facts = collect_message_facts("HELLO world");
        // 5 uppercase / 10 alpha = 0.5
        assert!((facts.uppercase_ratio - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn punctuation_density_calculation() {
        let facts = collect_message_facts("Hi!!!");
        // 3 punctuation / 5 chars = 0.6
        assert!((facts.punctuation_density - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn emoji_counting() {
        // U+1F600 is a grinning face emoji
        let facts = collect_message_facts("Hello \u{1F600}\u{1F600}");
        assert_eq!(facts.emoji_count, 2);
    }

    #[test]
    fn interaction_facts_basic() {
        let facts = collect_interaction_facts(60, 5, true, false, 3);
        assert_eq!(facts.seconds_since_last_message, 60);
        assert_eq!(facts.session_turn_number, 5);
        assert!(facts.task_completed);
        assert!(!facts.task_failed);
        assert_eq!(facts.tool_calls_count, 3);
        assert!(!facts.topic_shifted); // always false for now
    }
}
