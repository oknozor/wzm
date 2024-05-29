use smithay::utils::{Logical, Point};

#[derive(Debug, PartialEq, Eq)]
pub enum KeyAction {
    ScaleUp,
    ScaleDown,
    RotateOutput,
    Screen(usize),
    ToggleTint,
    TogglePreview,
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
    CloseWindow,
    Quit,
    None,
    ToggleResize,
    ResizeLeft,
    ResizeRight,
    ResizeUp,
    ResizeDown,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn advance_point(&self, p: &mut Point<f64, Logical>) {
        match self {
            Direction::Left => p.x -= 1.0,
            Direction::Right => p.x += 1.0,
            Direction::Up => p.y -= 1.0,
            Direction::Down => p.y += 1.0,
        }
    }
}
