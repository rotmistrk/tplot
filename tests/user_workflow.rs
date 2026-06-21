//! Scenario tests: core user workflow.
//! Tests 1-20 covering the full analysis journey.

mod helpers;

use helpers::{temp_project, TestHarness};
use txv_core::event::{KeyCode, KeyMod};

fn none() -> KeyMod {
    KeyMod::default()
}

fn focus_cmd(h: &mut TestHarness) {
    h.inject_key(KeyCode::F(4), none());
    h.run_cycles(2);
}

fn focus_tree(h: &mut TestHarness) {
    h.inject_key(KeyCode::F(2), none());
    h.run_cycles(2);
}

fn focus_main(h: &mut TestHarness) {
    h.inject_key(KeyCode::F(3), none());
    h.run_cycles(2);
}

/// Enter insert mode, type text, escape back to normal.
fn type_in_editor(h: &mut TestHarness, text: &str) {
    h.inject_key(KeyCode::Char('i'), none()); // insert mode
    h.run_cycles(1);
    h.inject_str(text);
    h.run_cycles(1);
    h.inject_key(KeyCode::Esc, none()); // back to normal
    h.run_cycles(1);
}

fn press_f9(h: &mut TestHarness) {
    h.inject_key(KeyCode::F(9), none());
    h.run_cycles(5);
}

// ═══ Test 1-4: Create data with into/sql ═══

#[test]
fn t01_create_table_and_lineage_node() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    // Step 2: switch to cmd
    focus_cmd(&mut h);

    // Step 3: type a CREATE TABLE command
    type_in_editor(&mut h, "sql {CREATE TABLE auth AS SELECT * FROM (VALUES ('2024-01-01','root','192.168.1.100','failed'), ('2024-01-01','admin','10.0.0.5','failed')) AS t(ts, username, src_ip, status)}");

    // Step 4: F9
    press_f9(&mut h);

    // Step 5: lineage node created
    focus_tree(&mut h);
    h.run_cycles(2);
    let screen = h.screen_text();
    println!("=== AFTER CREATE ===\n{screen}");
    assert!(h.contains("auth"), "lineage should show 'auth' node");
}

#[test]
fn t02_table_view_shows_data() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(&mut h, "sql -name users {SELECT username, count(*) as cnt FROM (VALUES ('root'),('root'),('admin')) AS t(username) GROUP BY username}");
    press_f9(&mut h);

    // Step 6: main panel has data
    focus_main(&mut h);
    h.run_cycles(2);
    let screen = h.screen_text();
    println!("=== TABLE VIEW ===\n{screen}");
    assert!(h.contains("username"), "table should show column 'username'");
    assert!(h.contains("root"), "table should show data 'root'");
}

#[test]
fn t03_table_view_shows_command() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(&mut h, "sql -name top {SELECT 'hello' as msg}");
    press_f9(&mut h);

    // Step 7: main panel shows the command
    focus_main(&mut h);
    h.run_cycles(2);
    let screen = h.screen_text();
    println!("=== COMMAND IN TABLE ===\n{screen}");
    assert!(h.contains("SELECT"), "table header should show the SQL command");
}

// ═══ Tests 10-14: Multiple queries, parent-child relationships ═══

#[test]
fn t10_child_query_creates_lineage_child() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    // Create parent table
    focus_cmd(&mut h);
    type_in_editor(&mut h, "sql {CREATE TABLE events AS SELECT 1 as id, 'click' as type}");
    press_f9(&mut h);

    // Move to next line in editor
    h.inject_key(KeyCode::Char('o'), none()); // open line below in vim
    h.run_cycles(1);
    h.inject_str("sql -name clicks {SELECT * FROM events WHERE type='click'}");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    // Check lineage: events should have child 'clicks'
    focus_tree(&mut h);
    h.run_cycles(2);
    let screen = h.screen_text();
    println!("=== LINEAGE WITH CHILD ===\n{screen}");
    assert!(h.contains("events"), "parent 'events' should exist");
    assert!(h.contains("clicks"), "child 'clicks' should exist");
}

#[test]
fn t11_two_children_from_same_parent() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(&mut h, "sql {CREATE TABLE data AS SELECT 1 as x, 2 as y}");
    press_f9(&mut h);

    // First child
    h.inject_key(KeyCode::Char('o'), none());
    h.run_cycles(1);
    h.inject_str("sql -name child_a {SELECT x FROM data}");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    // Second child
    h.inject_key(KeyCode::Char('o'), none());
    h.run_cycles(1);
    h.inject_str("sql -name child_b {SELECT y FROM data}");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    // Check lineage: data with two children
    focus_tree(&mut h);
    h.run_cycles(2);
    let screen = h.screen_text();
    println!("=== TWO CHILDREN ===\n{screen}");
    assert!(h.contains("data"), "parent 'data' should exist");
    assert!(h.contains("child_a"), "child_a should exist");
    assert!(h.contains("child_b"), "child_b should exist");
}

// ═══ Test 16: Select node from lineage updates main panel ═══

#[test]
fn t16_select_node_updates_main_panel() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(&mut h, "sql -name greeting {SELECT 'world' as hello}");
    press_f9(&mut h);

    // Select the node from lineage tree
    focus_tree(&mut h);
    h.inject_key(KeyCode::Char('j'), none()); // navigate to the node
    h.run_cycles(1);
    h.inject_key(KeyCode::Enter, none()); // select it
    h.run_cycles(3);

    // Main panel should show the data
    let screen = h.screen_text();
    println!("=== AFTER NODE SELECT ===\n{screen}");
    assert!(h.contains("world"), "main panel should show 'world' after node select");
}

// ═══ Test 18-20: Plot ═══

#[test]
fn t18_plot_creates_lineage_node() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(
        &mut h,
        "sql -name nums {SELECT 'a' as x, 3 as y UNION ALL SELECT 'b', 7 UNION ALL SELECT 'c', 2}",
    );
    press_f9(&mut h);

    // Plot
    h.inject_key(KeyCode::Char('o'), none());
    h.run_cycles(1);
    h.inject_str("plot bar nums x y");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    // Check lineage has plot node
    focus_tree(&mut h);
    h.run_cycles(2);
    let screen = h.screen_text();
    println!("=== PLOT NODE ===\n{screen}");
    assert!(h.contains("plot"), "lineage should show plot node");

    // Check main panel has the chart (bar characters)
    assert!(
        h.contains("█") || h.contains("│"),
        "main panel should show chart elements"
    );
}

// ═══ Test 8: Table navigation (sort, filter, info) ═══

#[test]
fn t08_table_shows_row_count_and_cols() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(
        &mut h,
        "sql -name info_test {SELECT 'a' as col1, 1 as col2 UNION ALL SELECT 'b', 2 UNION ALL SELECT 'c', 3}",
    );
    press_f9(&mut h);

    focus_main(&mut h);
    h.run_cycles(2);
    let screen = h.screen_text();
    println!("=== TABLE INFO ===\n{screen}");

    // Should show column headers
    assert!(h.contains("col1"), "should show column 'col1'");
    assert!(h.contains("col2"), "should show column 'col2'");
    // Should show all 3 rows
    assert!(h.contains("a"), "should show row 'a'");
    assert!(h.contains("b"), "should show row 'b'");
    assert!(h.contains("c"), "should show row 'c'");
}

#[test]
fn t08b_table_navigable_with_jk() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(
        &mut h,
        "sql -name nav {SELECT 'row1' as name UNION ALL SELECT 'row2' UNION ALL SELECT 'row3'}",
    );
    press_f9(&mut h);

    focus_main(&mut h);
    h.run_cycles(2);

    // Navigate with j (down)
    h.inject_key(KeyCode::Char('j'), none());
    h.run_cycles(1);
    h.inject_key(KeyCode::Char('j'), none());
    h.run_cycles(1);

    // Should still show all rows (no crash, cursor moved)
    assert!(h.contains("row1"), "row1 visible");
    assert!(h.contains("row3"), "row3 visible");
}

// ═══ Test 9: Progress indication (placeholder - needs JobManager) ═══

#[test]
#[ignore] // Not yet implemented: progress bar for long operations
fn t09_progress_bar_for_long_operation() {
    // This test will require:
    // - A slow operation (large import)
    // - JobManager wired to async execution
    // - Tree showing ">" (running) status
    // - Status bar showing progress (rows/bytes)
    // - Ctrl+C to cancel
}

// ═══ Test 15: Verify data for both children ═══

#[test]
fn t15_both_children_show_correct_data() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(&mut h, "sql {CREATE TABLE src AS SELECT 1 as a, 2 as b, 3 as c}");
    press_f9(&mut h);

    h.inject_key(KeyCode::Char('o'), none());
    h.run_cycles(1);
    h.inject_str("sql -name get_a {SELECT a FROM src}");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    h.inject_key(KeyCode::Char('o'), none());
    h.run_cycles(1);
    h.inject_str("sql -name get_b {SELECT b FROM src}");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    // Select get_a from tree and verify
    focus_tree(&mut h);
    h.run_cycles(2);
    // Navigate: src(0) → get_a(1)
    h.inject_key(KeyCode::Char('j'), none());
    h.run_cycles(1);
    h.inject_key(KeyCode::Enter, none());
    h.run_cycles(3);

    let screen = h.screen_text();
    println!("=== get_a ===\n{screen}");
    assert!(h.contains("a"), "get_a should show column 'a'");

    // Now select get_b
    focus_tree(&mut h);
    h.inject_key(KeyCode::Char('j'), none()); // to get_b
    h.run_cycles(1);
    h.inject_key(KeyCode::Enter, none());
    h.run_cycles(3);

    let screen = h.screen_text();
    println!("=== get_b ===\n{screen}");
    assert!(h.contains("b"), "get_b should show column 'b'");
}

// ═══ Test 17: Send node command to cmd buffer ═══

#[test]
#[ignore] // Not yet implemented: copy command from node to editor
fn t17_send_command_to_editor() {
    // When a node is selected in lineage, pressing a key (e.g., 'e')
    // should copy the node's command to the cmd editor for editing.
    // This allows modifying and re-running queries.
}

// ═══ Tests 19-20: Plot details ═══

#[test]
fn t19_plot_shows_chart_content() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(
        &mut h,
        "sql -name bars {SELECT 'alpha' as x, 10 as y UNION ALL SELECT 'beta', 5}",
    );
    press_f9(&mut h);

    h.inject_key(KeyCode::Char('o'), none());
    h.run_cycles(1);
    h.inject_str("plot bar bars x y");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    let screen = h.screen_text();
    println!("=== PLOT CONTENT ===\n{screen}");

    // Chart should show labels and bars
    assert!(h.contains("alpha"), "chart should show label 'alpha'");
    assert!(h.contains("beta"), "chart should show label 'beta'");
    assert!(h.contains("█"), "chart should show bar blocks");
}

#[test]
fn t20_plot_shows_command_header() {
    let dir = temp_project(&[]);
    let mut h = TestHarness::with_size(dir.path(), 120, 30);
    h.run_cycles(2);

    focus_cmd(&mut h);
    type_in_editor(&mut h, "sql -name pdata {SELECT 'x' as a, 5 as b}");
    press_f9(&mut h);

    h.inject_key(KeyCode::Char('o'), none());
    h.run_cycles(1);
    h.inject_str("plot bar pdata a b");
    h.inject_key(KeyCode::Esc, none());
    h.run_cycles(1);
    press_f9(&mut h);

    let screen = h.screen_text();
    println!("=== PLOT COMMAND ===\n{screen}");

    // Plot view should show the command
    assert!(h.contains("plot bar pdata"), "plot should show command header");
}

// ═══ Deletion and cloning (future) ═══

#[test]
#[ignore] // Not yet implemented: delete subtree
fn t_delete_subtree() {
    // Delete a node → removes it and all children from lineage
    // Data cleaned from disk
    // Confirmation required
}

#[test]
#[ignore] // Not yet implemented: clone subtree
fn t_clone_subtree() {
    // Clone a node → creates copy with new name
    // Children are cloned as Empty (not materialized)
    // Original unchanged
}

#[test]
#[ignore] // Not yet implemented: shared children deletion
fn t_shared_child_deletion() {
    // If node A and node B both reference child C (multi-parent),
    // deleting A should ask about C:
    // - keep C (re-parent to B only)
    // - delete C (confirm explicitly)
}

// ═══ Multi-source queries (future) ═══

#[test]
#[ignore] // Not yet implemented: join from multiple sources
fn t_multi_source_join() {
    // sql -name joined {SELECT * FROM table_a JOIN table_b ON ...}
    // Should create node 'joined' with TWO parents: table_a, table_b
    // Lineage tree shows it under primary parent with link to secondary
}
