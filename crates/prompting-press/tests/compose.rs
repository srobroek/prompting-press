//! US4 composition contract (spec 008 reshape of spec 003, T021; FR-012/FR-013).
//!
//! Post-reshape, `Composition` aggregates **`Prompt` objects** (not names). `resolve()` takes
//! no `Registry`. Otherwise the contract is unchanged: an explicit ordered sequence that
//! resolves to an ordered `Vec<Message>` in append order.
//!
//! - **V4.1** N entries over constructed prompts, each with valid vars → `resolve` yields
//!   exactly N `Message`s in APPEND ORDER, each `text` = that prompt rendered with its vars
//!   and each `role` = that prompt definition's role (SC-008).
//! - **V4.2** one entry's vars violate a validator → the failing operation (`append`)
//!   returns `Err(ConsumerError::Validation(..))` naming the field; no partial `Vec`.
//! - **V4.3** a fragment rendered with its own vars, its `.text` passed as a declared
//!   variable into a parent prompt → composition-by-value works (US4 scenario 4).
//! - **V4.4** an empty `Composition` → `resolve()` returns `Ok(vec![])` (edge case F7).

use garde::Validate;
use prompting_press::error::code;
use prompting_press::{Composition, ConsumerError, Message, Prompt};
use prompting_press_core::GuardConfig;
use serde::Serialize;

/// Greeting vars (system prompt) — `name`, length-bounded.
#[derive(Debug, Serialize, Validate)]
struct GreetVars {
    #[garde(length(min = 1, max = 20))]
    name: String,
}

/// Question vars (user prompt) — `topic`, length-bounded.
#[derive(Debug, Serialize, Validate)]
struct AskVars {
    #[garde(length(min = 1, max = 50))]
    topic: String,
}

/// Answer vars (assistant prompt) — `answer`, length-bounded.
#[derive(Debug, Serialize, Validate)]
struct AnswerVars {
    #[garde(length(min = 1, max = 50))]
    answer: String,
}

/// Build three prompts: one per role (system / user / assistant).
fn three_prompts() -> (Prompt, Prompt, Prompt) {
    let greet = Prompt::from_json(
        r#"{
        "name": "greet",
        "role": "system",
        "body": "You are talking to {{ name }}.",
        "variables": { "name": { "type": "string", "origin": "trusted" } }
    }"#,
    )
    .expect("valid greet prompt");

    let ask = Prompt::from_json(
        r#"{
        "name": "ask",
        "role": "user",
        "body": "Tell me about {{ topic }}.",
        "variables": { "topic": { "type": "string", "origin": "trusted" } }
    }"#,
    )
    .expect("valid ask prompt");

    let answer = Prompt::from_json(
        r#"{
        "name": "answer",
        "role": "assistant",
        "body": "Here is what I know: {{ answer }}",
        "variables": { "answer": { "type": "string", "origin": "trusted" } }
    }"#,
    )
    .expect("valid answer prompt");

    (greet, ask, answer)
}

/// V4.1 — N ordered entries resolve to exactly N messages, in append order, each rendered
/// with its own vars and tagged with its prompt's role (SC-008).
#[test]
fn ordered_composition_resolves_n_to_n() {
    let (greet, ask, answer) = three_prompts();

    let mut comp = Composition::new();
    comp.append(
        &greet,
        &GreetVars {
            name: "Ada".to_string(),
        },
        None,
    )
    .expect("greet vars valid");
    comp.append(
        &ask,
        &AskVars {
            topic: "rust".to_string(),
        },
        None,
    )
    .expect("ask vars valid");
    comp.append(
        &answer,
        &AnswerVars {
            answer: "it is fast".to_string(),
        },
        None,
    )
    .expect("answer vars valid");

    let messages = comp.resolve().expect("all entries valid → resolves");

    // Exactly N (3) messages, in APPEND ORDER (SC-008).
    assert_eq!(messages.len(), 3, "N entries → exactly N messages");

    let expected: [(&str, &str); 3] = [
        ("system", "You are talking to Ada."),
        ("user", "Tell me about rust."),
        ("assistant", "Here is what I know: it is fast"),
    ];
    for (msg, (role, text)) in messages.iter().zip(expected.iter()) {
        assert_eq!(&msg.role, role, "role matches the prompt def's role");
        assert_eq!(&msg.text, text, "text = prompt rendered with its own vars");
    }

    // Cross-check: the first message must equal a direct render of the greet prompt.
    let direct = greet
        .render(
            &GreetVars {
                name: "Ada".to_string(),
            },
            None,
            &GuardConfig::default(),
        )
        .expect("direct render");
    assert_eq!(messages[0].text, direct.text);
    assert_eq!(messages[0].role, greet.role().to_string());
}

/// V4.2 — one entry's vars violate a validator → the failing `append` returns
/// `Err(ConsumerError::Validation(..))` naming the field; no partial `Vec` produced.
#[test]
fn invalid_entry_vars_error_no_partial_success() {
    let (greet, ask, _answer) = three_prompts();

    let mut comp = Composition::new();
    // First entry is valid.
    comp.append(
        &greet,
        &GreetVars {
            name: "Ada".to_string(),
        },
        None,
    )
    .expect("first entry valid");

    // Second entry's vars violate `length(min = 1)` → append fails fast.
    let err = comp
        .append(
            &ask,
            &AskVars {
                topic: String::new(),
            },
            None,
        )
        .expect_err("empty topic must be rejected at append");

    match err {
        ConsumerError::Validation(rows) => {
            assert_eq!(rows.len(), 1, "exactly the one offending field");
            assert_eq!(rows[0].field, "topic", "the failing field is named");
            assert_eq!(rows[0].code, code::VALIDATION);
        }
        other => panic!("expected ConsumerError::Validation, got {other:?}"),
    }

    // No-partial-as-success: only the valid entry remains.
    let messages = comp.resolve().expect("only the valid entry remains");
    assert_eq!(messages.len(), 1, "only the successfully-appended entry");
    assert_eq!(messages[0].role, "system");
    assert_eq!(messages[0].text, "You are talking to Ada.");
}

/// V4.3 — fragment-by-composition: render a fragment with its own vars, then pass its
/// `.text` as a declared variable's value into a parent prompt. No template include.
#[test]
fn fragment_by_value_into_parent() {
    let fragment_prompt = Prompt::from_json(
        r#"{
        "name": "fragment",
        "role": "user",
        "body": "the {{ adjective }} fox",
        "variables": { "adjective": { "type": "string", "origin": "trusted" } }
    }"#,
    )
    .expect("valid fragment prompt");

    let parent_prompt = Prompt::from_json(
        r#"{
        "name": "parent",
        "role": "user",
        "body": "Story: {{ fragment }} jumped.",
        "variables": { "fragment": { "type": "string", "origin": "trusted" } }
    }"#,
    )
    .expect("valid parent prompt");

    #[derive(Debug, Serialize, Validate)]
    struct FragVars {
        #[garde(length(min = 1))]
        adjective: String,
    }
    #[derive(Debug, Serialize, Validate)]
    struct ParentVars {
        #[garde(length(min = 1))]
        fragment: String,
    }

    // 1. Render the fragment with its OWN vars.
    let frag = fragment_prompt
        .render(
            &FragVars {
                adjective: "quick".to_string(),
            },
            None,
            &GuardConfig::default(),
        )
        .expect("fragment renders");
    assert_eq!(frag.text, "the quick fox");

    // 2. Pass the fragment's text as a value to the parent — no include, pure by-value.
    let parent = parent_prompt
        .render(
            &ParentVars {
                fragment: frag.text.clone(),
            },
            None,
            &GuardConfig::default(),
        )
        .expect("parent renders with the fragment value");
    assert_eq!(parent.text, "Story: the quick fox jumped.");

    // 3. The same pattern via Composition.
    let mut comp = Composition::new();
    comp.append(
        &fragment_prompt,
        &FragVars {
            adjective: "quick".to_string(),
        },
        None,
    )
    .expect("fragment vars valid");
    comp.append(
        &parent_prompt,
        &ParentVars {
            fragment: frag.text.clone(),
        },
        None,
    )
    .expect("parent vars valid");
    let messages = comp.resolve().expect("composition resolves");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, "the quick fox");
    assert_eq!(messages[1].text, "Story: the quick fox jumped.");
}

/// V4.4 — an empty `Composition` resolves to `Ok(vec![])` (edge case F7), never a panic.
#[test]
fn empty_composition_resolves_to_empty_vec() {
    let comp = Composition::new();
    let messages: Vec<Message> = comp.resolve().expect("empty composition is a pass");
    assert!(messages.is_empty(), "empty composition → empty Vec");
}
