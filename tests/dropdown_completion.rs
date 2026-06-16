//! Scenario test: dropdown completion appears near the cursor and is navigable.
//! The dropdown bottom border must be directly above the prompt line,
//! and the dropdown must be horizontally near the cursor.

mod helpers;

use helpers::{temp_project, TestHarness};
use txv_core::event::{KeyCode, KeyMod};

fn none() -> KeyMod {
    KeyMod::default()
}

fn focus_repl(h: &mut TestHarness) {
    h.inject_key(KeyCode::F(4), none());
    h.run_cycles(10);
}

/// Find the screen row containing the prompt "tplot>"
fn prompt_row(h: &TestHarness, height: u16) -> u16 {
    for y in (0..height).rev() {
        if h.row(y).contains("tplot>") {
            return y;
        }
    }
    panic!("prompt not found");
}

/// Find the first screen row that contains the given text.
fn row_containing(h: &TestHarness, text: &str, height: u16) -> Option<u16> {
    for y in 0..height {
        if h.row(y).contains(text) {
            return Some(y);
        }
    }
    None
}

/// The dropdown bottom border ("└") must be on the row directly above the prompt.
#[test]
fn dropdown_bottom_is_one_above_prompt() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 160, 40);
    h.run_cycles(10);
    focus_repl(&mut h);

    h.inject_str("into x ");
    h.run_cycles(1);
    h.inject_key(KeyCode::Tab, none());
    h.run_cycles(10);

    let height = 40;
    let pr = prompt_row(&h, height);

    // Find the dropdown bottom border (row with └...┘)
    let mut dropdown_bottom: Option<u16> = None;
    for y in 0..height {
        let r = h.row(y);
        if r.contains("└") && r.contains("┘") {
            dropdown_bottom = Some(y);
        }
    }
    let dropdown_bottom = dropdown_bottom.expect("dropdown border not found");

    // Print context for debugging
    for y in dropdown_bottom.saturating_sub(1)..=pr.min(height - 1) {
        println!("row {y:2}: {}", h.row(y));
    }
    println!("prompt_row={pr}, dropdown_bottom={dropdown_bottom}");

    assert_eq!(
        dropdown_bottom,
        pr - 1,
        "dropdown bottom must be 1 row above the prompt"
    );
}

/// The dropdown left edge must be near the cursor (within 5 cols).
#[test]
fn dropdown_horizontally_near_cursor() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 160, 40);
    h.run_cycles(10);
    focus_repl(&mut h);

    h.inject_str("into x ");
    h.run_cycles(1);
    h.inject_key(KeyCode::Tab, none());
    h.run_cycles(10);

    let height = 40;

    // Find the dropdown top border "┌"
    let top_row = row_containing(&h, "┌", height).expect("dropdown top not found");
    let top_line = h.row(top_row);
    let dropdown_x = top_line.find('┌').unwrap();

    // Find cursor position: "tplot> into x " — cursor is after the space
    let pr = prompt_row(&h, height);
    let prompt_line = h.row(pr);
    // Cursor is at end of "into x " in the prompt
    let cursor_x = prompt_line.find("into x ").map(|p| p + 7).unwrap_or(0);

    println!("dropdown_x={dropdown_x}, cursor_x={cursor_x}");
    println!("top: {top_line}");
    println!("prompt: {prompt_line}");

    let diff = (dropdown_x as i32 - cursor_x as i32).unsigned_abs();
    assert!(
        diff < 5,
        "dropdown left edge ({dropdown_x}) too far from cursor ({cursor_x}), diff={diff}"
    );
}

/// After Enter on the dropdown, the selected item is applied to the input.
#[test]
fn enter_applies_first_item() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 160, 40);
    h.run_cycles(10);
    focus_repl(&mut h);

    h.inject_str("into x ");
    h.run_cycles(1);
    h.inject_key(KeyCode::Tab, none());
    h.run_cycles(10);

    // Enter should apply first item "-file"
    h.inject_key(KeyCode::Enter, none());
    h.run_cycles(10);

    let pr = prompt_row(&h, 40);
    let line = h.row(pr);
    println!("after enter: {line}");
    assert!(
        line.contains("-file"),
        "first item '-file' not applied to input: {line}"
    );
}

/// Down+Enter selects the second item.
#[test]
fn down_enter_applies_second_item() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 160, 40);
    h.run_cycles(10);
    focus_repl(&mut h);

    h.inject_str("into x ");
    h.run_cycles(1);
    h.inject_key(KeyCode::Tab, none());
    h.run_cycles(10);

    h.inject_key(KeyCode::Down, none());
    h.run_cycles(1);
    h.inject_key(KeyCode::Enter, none());
    h.run_cycles(10);

    let pr = prompt_row(&h, 40);
    let line = h.row(pr);
    println!("after down+enter: {line}");
    assert!(line.contains("-source"), "second item '-source' not applied: {line}");
}

/// Esc dismisses the dropdown without changing input.
#[test]
fn esc_dismisses_dropdown() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 160, 40);
    h.run_cycles(10);
    focus_repl(&mut h);

    h.inject_str("into x ");
    h.run_cycles(1);
    h.inject_key(KeyCode::Tab, none());
    h.run_cycles(10);
    assert!(h.contains("-file"), "dropdown should be visible");

    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(10);

    // Dropdown should be gone
    assert!(!h.contains("┌"), "dropdown should be dismissed");
    // Input unchanged
    let pr = prompt_row(&h, 40);
    let line = h.row(pr);
    assert!(line.contains("into x"), "input should be unchanged: {line}");
}
