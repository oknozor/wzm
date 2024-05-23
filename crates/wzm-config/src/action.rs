#[derive(Debug, PartialEq, Eq)]
pub enum KeyAction {
    ScaleUp,
    ScaleDown,
    RotateOutput,
    Screen(usize),
    ToggleTint,
    TogglePreview,
    ToggleDecorations,
    ToggleFullScreenWindow,
    ToggleFullScreenContainer,
    MoveWindow(Direction),
    MoveContainer(Direction),
    MoveFocus(Direction),
    Run(String, Vec<(String, String)>),
    MoveToWorkspace(u8),
    LayoutVertical,
    LayoutHorizontal,
    ToggleFloating,
    VtSwitch(i32),
    Close,
    Quit,
    None,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
