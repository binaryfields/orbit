//! Launcher window lifecycle: show -> activate -> settle.
//!
//! macOS accessory-app quirks: the window isn't focusable for a frame after
//! Visible(true), and reports spurious unfocused frames before activation.
//! State machine:
//!
//!   Hidden -> Activating (waiting for focus) -> Interactive (focus loss now
//!   means the user clicked away)

const ACTIVATE_DELAY_FRAMES: u32 = 1;
const ACTIVATION_TIMEOUT_FRAMES: u32 = 5;

#[derive(Debug, PartialEq)]
pub enum Window {
    Hidden,
    Activating { frames: u32 },
    Interactive,
}

#[derive(Debug, PartialEq)]
pub enum Step {
    Idle,
    Settle { activate: bool },
    Dismiss,
}

impl Window {
    pub fn advance(&mut self, focused: Option<bool>) -> Step {
        match *self {
            Window::Hidden => Step::Idle,
            Window::Activating { frames } => {
                if focused == Some(true) {
                    *self = Window::Interactive;
                    Step::Idle
                } else if frames >= ACTIVATION_TIMEOUT_FRAMES {
                    *self = Window::Interactive;
                    interactive_step(focused)
                } else {
                    *self = Window::Activating { frames: frames + 1 };
                    Step::Settle {
                        activate: frames == ACTIVATE_DELAY_FRAMES,
                    }
                }
            }
            Window::Interactive => interactive_step(focused),
        }
    }

    pub fn is_visible(&self) -> bool {
        !matches!(self, Window::Hidden)
    }

    pub fn just_shown(&self) -> bool {
        matches!(self, Window::Activating { frames } if *frames <= 1)
    }
}

fn interactive_step(focused: Option<bool>) -> Step {
    if focused == Some(false) {
        Step::Dismiss
    } else {
        Step::Idle
    }
}
