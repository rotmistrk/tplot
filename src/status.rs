//! Status bar construction with key bindings.

use txv_core::event::{CommandId, KeyCode, KeyEvent, KeyMod};
use txv_core::status_bar::{StatusBar, StatusSlot};
use txv_widgets::tiled_workspace::commands::{CM_TW_FOCUS_PANEL, CM_TW_LAYOUT_CYCLE, CM_TW_TAB_CLOSE, CM_TW_ZOOM};
use txv_widgets::tiled_workspace::TiledWorkspace;
use txv_widgets::{ConfirmView, InputLine, KeyLabelView, MessageView, ModalKey};

/// Base for tplot application commands (above txv-widgets range).
const CM_APP_BASE: CommandId = txv_core::commands::CM_TXV_MAX + 1;

/// Application command IDs.
pub(crate) const CM_APP_QUIT: CommandId = CM_APP_BASE;
pub(crate) const CM_SHOW_HELP: CommandId = CM_APP_BASE + 1;
/// Activate confirmation dialog. Payload: prompt String.
pub(crate) const CM_CONFIRM_ACTIVATE: CommandId = CM_APP_BASE + 2;
/// Confirmation response. Payload: char ('y'/'n'/'c').
pub(crate) const CM_CONFIRM_RESPONSE: CommandId = CM_APP_BASE + 3;
/// M-x command submitted. Payload: String.
pub(crate) const CM_EXECUTE_COMMAND: CommandId = CM_APP_BASE + 4;
/// Enter command mode programmatically.
pub(crate) const CM_COMMAND_MODE: CommandId = CM_APP_BASE + 5;
/// Prefill command line. Payload: String.
pub(crate) const CM_COMMAND_PREFILL: CommandId = CM_APP_BASE + 6;

pub fn build_status_bar(desktop: &TiledWorkspace) -> StatusBar {
    let mut bar = StatusBar::new();

    // Register workspace default bindings (tab switching, panel nav).
    for (k, command, _payload) in desktop.default_bindings() {
        bar.add(StatusSlot::new(Box::new(KeyLabelView::new(k, command, ""))));
    }

    // Visible app bindings.
    add_visible_bindings(&mut bar);

    // M-x command line.
    let input = InputLine::new()
        .with_command(CM_EXECUTE_COMMAND)
        .with_prefill_command(CM_COMMAND_PREFILL);
    let command_line = ModalKey::new("M-x", ":")
        .trigger_key(alt('x'))
        .trigger_command(CM_COMMAND_MODE)
        .prefill_command(CM_COMMAND_PREFILL)
        .terminal_command(CM_EXECUTE_COMMAND)
        .add_child(Box::new(input));
    bar.add(StatusSlot::new(Box::new(command_line)).priority(9).stretch(1));

    // Confirm dialog (y/n prompt).
    bar.add(StatusSlot::new(Box::new(ConfirmView::new(CM_CONFIRM_ACTIVATE, CM_CONFIRM_RESPONSE))).priority(10));

    // Message display (shows info/warn/error with timeout).
    bar.add(StatusSlot::new(Box::new(MessageView::new(5))).priority(8).stretch(1));

    // Hidden panel focus bindings.
    add_panel_focus(&mut bar);

    // Hidden misc.
    add_misc_bindings(&mut bar);

    bar
}

fn add_visible_bindings(bar: &mut StatusBar) {
    use crate::views::cmd_editor::{CM_EXEC_BUFFER, CM_EXEC_LINE};
    bar.add(
        StatusSlot::new(Box::new(KeyLabelView::new(
            key(KeyCode::F(1)),
            CM_SHOW_HELP,
            "~F1~:Help",
        )))
        .priority(6),
    );
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(key(KeyCode::F(5)), CM_TW_ZOOM, "~F5~:Zoom"))).priority(5));
    bar.add(
        StatusSlot::new(Box::new(KeyLabelView::new(
            key(KeyCode::F(9)),
            CM_EXEC_LINE,
            "~F9~:Run",
        )))
        .priority(7),
    );
    bar.add(
        StatusSlot::new(Box::new(KeyLabelView::new(
            key(KeyCode::F(10)),
            CM_EXEC_BUFFER,
            "~F10~:All",
        )))
        .priority(5),
    );
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(ctrl('q'), CM_APP_QUIT, "~C-q~:Quit"))).priority(9));
}

fn add_panel_focus(bar: &mut StatusBar) {
    // F2=tree, F3=center, F4=tools
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(key(KeyCode::F(2)), CM_TW_FOCUS_PANEL, "~F2~:Tree").with_data(0),
    )));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(key(KeyCode::F(3)), CM_TW_FOCUS_PANEL, "~F3~:Main").with_data(1),
    )));
    bar.add(StatusSlot::new(Box::new(
        KeyLabelView::new(key(KeyCode::F(4)), CM_TW_FOCUS_PANEL, "~F4~:Tools").with_data(2),
    )));
}

fn add_misc_bindings(bar: &mut StatusBar) {
    // Alt-w close tab.
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(
        alt('w'),
        CM_TW_TAB_CLOSE,
        "",
    ))));
    // Alt-\ toggle layout.
    let alt_backslash = KeyEvent::new(KeyCode::Char('\\'), KeyMod::ALT);
    bar.add(StatusSlot::new(Box::new(KeyLabelView::new(
        alt_backslash,
        CM_TW_LAYOUT_CYCLE,
        "",
    ))));
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyMod::NONE)
}

fn ctrl(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyMod::CTRL)
}

fn alt(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyMod::ALT)
}
