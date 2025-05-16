use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustParseState {
    Start,
    Normal,
    FirstSlash,
    SingleLineComment,
    MultiLineComment,
    MultiLineCommentSawStar,
    StringDoubleQuotes,
    StringDoubleQuotesEscaped,
    CharSingleQuotes,
    CharSingleQuotesEscaped,
    End,
}
impl Start for RustParseState {
    fn start() -> Self {
        RustParseState::Start
    }
}
impl End for RustParseState {
    fn end() -> Self {
        RustParseState::End
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustParseAction {
    Nothing,
    CommentMightStart,
    ConfirmLineComment,
    ConfirmBlockComment,
    DismissPotential,
    MaybeEndBlock,
    ConfirmEndBlock,
    DecrementNest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RustCommentTrackState {
    nest_level: usize,
    start_pos: usize,
    in_line_comment: bool,
    potential_start_pos: Option<usize>,
}
impl Start for RustCommentTrackState {
    fn start() -> Self {
        RustCommentTrackState {
            nest_level: 0,
            start_pos: 0,
            in_line_comment: false,
            potential_start_pos: None,
        }
    }
}

fn rust_state_transition(
    from: RustParseState,
    current_char: Option<char>,
) -> (RustParseState, RustParseAction) {
    match current_char {
        Some(c) => match from {
            RustParseState::StringDoubleQuotes => match c {
                '"' => (RustParseState::Normal, RustParseAction::Nothing),
                '\\' => (
                    RustParseState::StringDoubleQuotesEscaped,
                    RustParseAction::Nothing,
                ),
                _ => (RustParseState::StringDoubleQuotes, RustParseAction::Nothing),
            },
            RustParseState::StringDoubleQuotesEscaped => {
                (RustParseState::StringDoubleQuotes, RustParseAction::Nothing)
            }
            RustParseState::CharSingleQuotes => match c {
                '\'' => (RustParseState::Normal, RustParseAction::Nothing),
                '\\' => (
                    RustParseState::CharSingleQuotesEscaped,
                    RustParseAction::Nothing,
                ),
                _ => (RustParseState::CharSingleQuotes, RustParseAction::Nothing),
            },
            RustParseState::CharSingleQuotesEscaped => {
                (RustParseState::CharSingleQuotes, RustParseAction::Nothing)
            }

            RustParseState::Start | RustParseState::Normal => match c {
                '/' => (
                    RustParseState::FirstSlash,
                    RustParseAction::CommentMightStart,
                ),
                '"' => (RustParseState::StringDoubleQuotes, RustParseAction::Nothing),
                '\'' => (RustParseState::CharSingleQuotes, RustParseAction::Nothing),
                _ => (RustParseState::Normal, RustParseAction::Nothing),
            },
            RustParseState::FirstSlash => match c {
                '/' => (
                    RustParseState::SingleLineComment,
                    RustParseAction::ConfirmLineComment,
                ),
                '*' => (
                    RustParseState::MultiLineComment,
                    RustParseAction::ConfirmBlockComment,
                ),
                _ => (RustParseState::Normal, RustParseAction::DismissPotential),
            },
            RustParseState::SingleLineComment => match c {
                '\n' => (RustParseState::Start, RustParseAction::ConfirmEndBlock),
                _ => (RustParseState::SingleLineComment, RustParseAction::Nothing),
            },
            RustParseState::MultiLineComment => match c {
                '*' => (
                    RustParseState::MultiLineCommentSawStar,
                    RustParseAction::MaybeEndBlock,
                ),
                '/' => (
                    RustParseState::FirstSlash,
                    RustParseAction::CommentMightStart,
                ),
                _ => (RustParseState::MultiLineComment, RustParseAction::Nothing),
            },
            RustParseState::MultiLineCommentSawStar => match c {
                '/' => (RustParseState::Normal, RustParseAction::DecrementNest),
                '*' => (
                    RustParseState::MultiLineCommentSawStar,
                    RustParseAction::Nothing,
                ),
                _ => (RustParseState::MultiLineComment, RustParseAction::Nothing),
            },
            RustParseState::End => (RustParseState::End, RustParseAction::Nothing),
        },
        None => match from {
            RustParseState::SingleLineComment => {
                (RustParseState::End, RustParseAction::ConfirmEndBlock)
            }
            _ => (RustParseState::End, RustParseAction::Nothing),
        },
    }
}

fn rust_do_action(
    action: RustParseAction,
    mut state: RustCommentTrackState,
    position: usize,
    mut matches: Vec<CommentMatch>,
) -> Result<(RustCommentTrackState, Vec<CommentMatch>), StripError> {
    match action {
        RustParseAction::Nothing => {}
        RustParseAction::CommentMightStart => {
            if state.nest_level == 0 && !state.in_line_comment {
                state.potential_start_pos = Some(position);
            } else if state.nest_level > 0 {
                state.potential_start_pos = Some(position);
            }
        }
        RustParseAction::ConfirmLineComment => {
            if state.nest_level == 0 && !state.in_line_comment {
                if let Some(slash_idx) = state.potential_start_pos {
                    state = RustCommentTrackState {
                        start_pos: slash_idx,
                        nest_level: 0,
                        in_line_comment: true,
                        potential_start_pos: None,
                    };
                } else {
                    state = RustCommentTrackState::start();
                }
            }
            state.potential_start_pos = None;
        }
        RustParseAction::ConfirmBlockComment => {
            if let Some(slash_idx) = state.potential_start_pos {
                if state.nest_level == 0 && !state.in_line_comment {
                    state = RustCommentTrackState {
                        start_pos: slash_idx,
                        nest_level: 1,
                        in_line_comment: false,
                        potential_start_pos: None,
                    };
                } else if !state.in_line_comment {
                    state.nest_level += 1;
                    state.potential_start_pos = None;
                }
            } else {
                state = RustCommentTrackState::start();
            }
        }
        RustParseAction::DismissPotential => {
            state.potential_start_pos = None;
        }
        RustParseAction::MaybeEndBlock => {}
        RustParseAction::DecrementNest | RustParseAction::ConfirmEndBlock => {
            if state.in_line_comment {
                matches.push(CommentMatch {
                    from: state.start_pos,
                    to: position,
                });
                state = RustCommentTrackState::start();
            } else if state.nest_level > 0 {
                if action == RustParseAction::DecrementNest {
                    state.nest_level -= 1;
                    if state.nest_level == 0 {
                        matches.push(CommentMatch {
                            from: state.start_pos,
                            to: position + 1,
                        });
                        state = RustCommentTrackState::start();
                    }
                }
            }
            state.potential_start_pos = None;
        }
    }
    Ok((state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl(input, rust_state_transition, rust_do_action)
}
