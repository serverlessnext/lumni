use super::key_event::KeyTrack;
use super::{ModalWindowType, WindowEvent};

pub const LEADER_KEY: char = ' ';

enum MatchOutcome {
    FullMatch(String),
    PartialMatch,
    NoMatch,
}

macro_rules! define_commands {
    ( $( $name:ident ),* ) => {
        #[allow(dead_code)]
        pub enum LeaderKeyCommand {
            $($name),*
        }

        impl LeaderKeyCommand {
            // Check if the provided string is a full or partial match to any command
            fn match_command(s: &str) -> MatchOutcome {
                let s = s.to_lowercase();
                // Check for full matches
                $(
                    if s == stringify!($name).to_lowercase() {
                        return MatchOutcome::FullMatch(s);
                    }
                )*

                // If no full match, check for partial matches
                $(
                    if stringify!($name).to_lowercase().starts_with(&s) {
                        return MatchOutcome::PartialMatch;
                    }
                )*
                MatchOutcome::NoMatch // No match found
            }
        }
    };
}

// <leader> + c -> config window
// NOTE: currently cant use <leader> + something that includes either "i" or "v"
// check note in key_event::update_previous_key_with_leader()
define_commands!(PC);

pub fn process_leader_key(key_track: &mut KeyTrack) -> Option<WindowEvent> {
    let leader_key_str = key_track.previous_key_str();

    match leader_key_str {
        Some(key_str) => match LeaderKeyCommand::match_command(key_str) {
            MatchOutcome::FullMatch(cmd) => {
                let window_event = match cmd.as_str() {
                    "pc" => Some(WindowEvent::Modal(ModalWindowType::Config)),
                    _ => None,
                };
                key_track.set_leader_key(false);
                return window_event;
            }
            MatchOutcome::PartialMatch => {}
            MatchOutcome::NoMatch => {
                key_track.set_leader_key(false);
            }
        },
        _ => {
            key_track.set_leader_key(false);
        }
    };
    None
}
