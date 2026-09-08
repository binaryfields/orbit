const ACTIVATION_TIMEOUT_FRAMES: u32 = 5;
const ACTIVATE_DELAY_FRAMES: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Idle,
    Repaint { activate: bool },
    Dismiss,
}

#[derive(Debug, PartialEq)]
pub enum WindowState {
    Hidden,
    Activating { frames: u32 },
    Interactive,
}

impl WindowState {
    pub fn advance(&mut self, focused: Option<bool>) -> Effect {
        match *self {
            WindowState::Hidden => Effect::Idle,
            WindowState::Activating { frames } => {
                if focused == Some(true) {
                    *self = WindowState::Interactive;
                    Effect::Idle
                } else if frames >= ACTIVATION_TIMEOUT_FRAMES {
                    *self = WindowState::Interactive;
                    interactive_effect(focused)
                } else {
                    *self = WindowState::Activating { frames: frames + 1 };
                    Effect::Repaint {
                        activate: frames == ACTIVATE_DELAY_FRAMES,
                    }
                }
            }
            WindowState::Interactive => interactive_effect(focused),
        }
    }

    pub fn is_visible(&self) -> bool {
        !matches!(self, WindowState::Hidden)
    }

    pub fn just_shown(&self) -> bool {
        matches!(self, WindowState::Activating { frames } if *frames <= 1)
    }
}

fn interactive_effect(focused: Option<bool>) -> Effect {
    if focused == Some(false) {
        Effect::Dismiss
    } else {
        Effect::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects(focused: Option<bool>) -> Vec<Effect> {
        let mut state = WindowState::Activating { frames: 0 };
        let mut out = Vec::new();
        while state != WindowState::Interactive && out.len() < 32 {
            out.push(state.advance(focused));
        }
        assert_eq!(state, WindowState::Interactive, "activation never settled");
        out
    }

    #[test]
    fn advance_by_focus() {
        use Effect::{Dismiss, Idle, Repaint};
        let warmup = [false, true, false, false, false].map(|activate| Repaint { activate });
        assert_eq!(effects(None), [&warmup[..], &[Idle]].concat());
        assert_eq!(effects(Some(false)), [&warmup[..], &[Dismiss]].concat());
        assert_eq!(effects(Some(true)), [Idle]);
    }

    #[test]
    fn just_shown_across_activation() {
        let mut state = WindowState::Activating { frames: 0 };
        let (mut filtered, mut frames) = (0, 0);
        while state != WindowState::Interactive {
            filtered += u32::from(state.just_shown());
            state.advance(None);
            frames += 1;
        }
        assert_eq!(filtered, 2);
        assert_eq!(frames, 6);
        assert!(filtered < frames, "filter outlives activation");
    }
}
