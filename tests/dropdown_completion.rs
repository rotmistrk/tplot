//! Test: dropdown completion popup position and navigation.

mod helpers;

use helpers::{temp_project, TestHarness};
use txv_core::event::{KeyCode, KeyMod};

fn none() -> KeyMod {
    KeyMod::default()
}

fn focus_repl(h: &mut TestHarness) {
    h.inject_key(KeyCode::F(4), none());
    h.run_cycles(2);
}

/// Find the row containing "tplot>" (the prompt line).
fn find_prompt_row(h: &TestHarness, height: u16) -> Option<u16> {
    for y in 0..height {
        if h.row(y).contains("tplot>") {
            return Some(y);
        }
    }
    None
}

/// Find the bottom border row of the dropdown (contains "└" and "┘").
fn find_dropdown_bottom(h: &TestHarness, height: u16) -> Option<u16> {
    for y in 0..height {
        let r = h.row(y);
        if r.contains("└") && r.contains("┘") && r.contains("/") {
            return Some(y);
        }
    }
    None
}

/// Find the column where the dropdown's left border "│" starts on a row with "-file".
fn find_dropdown_col(h: &TestHarness, height: u16) -> Option<u16> {
    for y in 0..height {
        let r = h.row(y);
        if r.contains("-file") {
            // Find the "│" before "-file"
            if let Some(pos) = r.find('│') {
                return Some(pos as u16);
            }
        }
    }
    None
}

/// The dropdown must appear directly above the prompt line,
/// and horizontally aligned near the cursor position.
#[test]
fn dropdown_position_above_cursor() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 160, 40);
    h.run_cycles(2);
    focus_repl(&mut h);

    // Type "into x " then Tab
    h.inject_str("into x ");
    h.run_cycles(1);
    h.inject_key(KeyCode::Tab, none());
    h.run_cycles(2);

    assert!(h.contains("-file"), "dropdown not showing");

    let height = 40;

    // Print all rows for debugging
    for y in 0..height {
        let r = h.row(y);
        if !r.trim().is_empty() {
            println!("row {y:2}: {r}");
        }
    }

    let prompt_row = find_prompt_row(&h, height).expect("prompt not found");
    let dropdown_bottom = find_dropdown_bottom(&h, height).expect("dropdown bottom not found");

    println!("prompt_row={prompt_row}, dropdown_bottom={dropdown_bottom}");

    // The dropdown bottom border must be exactly 1 row above the prompt.
    assert_eq!(
        dropdown_bottom,
        prompt_row - 1,
        "dropdown bottom ({dropdown_bottom}) should be 1 above prompt ({prompt_row})"
    );

    // Check horizontal position: dropdown should start near cursor.
    // "tplot> into x " = 15 chars, so cursor is around col 15 in the REPL.
    // The REPL is in the tools panel (right side in wide layout).
    // The dropdown left edge should be near the cursor column within the panel.
    let dropdown_col = find_dropdown_col(&h, height).expect("dropdown col not found");
    let prompt_line = h.row(prompt_row);
    let cursor_col = prompt_line.find("into x ").map(|p| p + 7).unwrap_or(0) as u16;

    println!("dropdown_col={dropdown_col}, cursor_col={cursor_col}");

    // Dropdown should be within reasonable range of cursor (not at col 0 or far right).
    let diff = (dropdown_col as i32 - cursor_col as i32).unsigned_abs();
    assert!(
        diff < 10,
        "dropdown col ({dropdown_col}) too far from cursor ({cursor_col}), diff={diff}"
    );
}
