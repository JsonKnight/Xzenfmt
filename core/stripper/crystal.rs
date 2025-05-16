use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Normal,
    SawHash,
    LineComment,
    MaybeBlockOpen,
    InBlockComment,
    InBlockCommentSawHash,
    StringDbl,
    StringDblEsc,
    StringSgl,
    End,
}
impl Start for ParseState {
    fn start() -> Self {
        ParseState::Normal
    }
}
impl End for ParseState {
    fn end() -> Self {
        ParseState::End
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseAction {
    Nothing,
    PotentialCommentHash,
    LineCommentEnd,
    MaybeBlockStart,
    ConfirmBlockStart,
    PotentialBlockEnd,
    BlockCommentEnd,
    ResetPotentialBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommentTrackState {
    nest_level: usize,
    start_pos: usize,
    is_line_comment: bool,
    potential_block_start_pos: Option<usize>,
    potential_line_start_pos: Option<usize>,
}
impl Start for CommentTrackState {
    fn start() -> Self {
        CommentTrackState {
            nest_level: 0,
            start_pos: 0,
            is_line_comment: false,
            potential_block_start_pos: None,
            potential_line_start_pos: None,
        }
    }
}

fn state_transition(from: ParseState, current_char: Option<char>) -> (ParseState, ParseAction) {
    match current_char {
        Some(c) => match from {
            ParseState::Normal => match c {
                '#' => (ParseState::SawHash, ParseAction::PotentialCommentHash),
                '{' => (ParseState::MaybeBlockOpen, ParseAction::MaybeBlockStart),
                '"' => (ParseState::StringDbl, ParseAction::Nothing),
                '\'' => (ParseState::StringSgl, ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::SawHash => match c {
                '{' => (ParseState::InBlockComment, ParseAction::ConfirmBlockStart),
                '\n' => (ParseState::Normal, ParseAction::LineCommentEnd),
                _ => (ParseState::LineComment, ParseAction::Nothing),
            },
            ParseState::LineComment => match c {
                '\n' => (ParseState::Normal, ParseAction::LineCommentEnd),
                _ => (ParseState::LineComment, ParseAction::Nothing),
            },
            ParseState::MaybeBlockOpen => match c {
                '#' => (ParseState::InBlockComment, ParseAction::ConfirmBlockStart),
                '{' => (ParseState::MaybeBlockOpen, ParseAction::MaybeBlockStart),
                _ => (ParseState::Normal, ParseAction::ResetPotentialBlock),
            },
            ParseState::InBlockComment => match c {
                '#' => (
                    ParseState::InBlockCommentSawHash,
                    ParseAction::PotentialBlockEnd,
                ),
                '{' => (ParseState::MaybeBlockOpen, ParseAction::MaybeBlockStart),
                _ => (ParseState::InBlockComment, ParseAction::Nothing),
            },
            ParseState::InBlockCommentSawHash => match c {
                '}' => (ParseState::Normal, ParseAction::BlockCommentEnd),
                '#' => (
                    ParseState::InBlockCommentSawHash,
                    ParseAction::PotentialBlockEnd,
                ),
                '{' => (ParseState::MaybeBlockOpen, ParseAction::MaybeBlockStart),
                _ => (ParseState::InBlockComment, ParseAction::Nothing),
            },
            ParseState::StringDbl => match c {
                '"' => (ParseState::Normal, ParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, ParseAction::Nothing),
                _ => (ParseState::StringDbl, ParseAction::Nothing),
            },
            ParseState::StringDblEsc => (ParseState::StringDbl, ParseAction::Nothing),
            ParseState::StringSgl => match c {
                '\'' => (ParseState::Normal, ParseAction::Nothing),
                _ => (ParseState::StringSgl, ParseAction::Nothing),
            },
            ParseState::End => (ParseState::End, ParseAction::Nothing),
        },
        None => match from {
            ParseState::SawHash | ParseState::LineComment => {
                (ParseState::End, ParseAction::LineCommentEnd)
            }
            ParseState::MaybeBlockOpen
            | ParseState::InBlockComment
            | ParseState::InBlockCommentSawHash => {
                (ParseState::End, ParseAction::ResetPotentialBlock)
            }
            _ => (ParseState::End, ParseAction::Nothing),
        },
    }
}

fn do_action(
    action: ParseAction,
    mut state: CommentTrackState,
    position: usize,
    mut matches: Vec<CommentMatch>,
) -> Result<(CommentTrackState, Vec<CommentMatch>), StripError> {
    match action {
        ParseAction::Nothing => {
            if let Some(start) = state.potential_line_start_pos {
                if !state.is_line_comment && state.nest_level == 0 {
                    state.is_line_comment = true;
                    state.start_pos = start;
                    state.potential_line_start_pos = None;
                    state.potential_block_start_pos = None;
                }
            }
        }
        ParseAction::PotentialCommentHash => {
            if state.nest_level == 0 && !state.is_line_comment {
                state.potential_line_start_pos = Some(position);
            }
            state.potential_block_start_pos = None;
        }
        ParseAction::LineCommentEnd => {
            if state.is_line_comment && state.nest_level == 0 {
                matches.push(CommentMatch {
                    from: state.start_pos,
                    to: position,
                });
                state = CommentTrackState::start();
            } else if let Some(from) = state.potential_line_start_pos {
                matches.push(CommentMatch { from, to: position });
                state = CommentTrackState::start();
            }
        }
        ParseAction::MaybeBlockStart => {
            if !(state.nest_level == 0 && state.is_line_comment) {
                state.potential_block_start_pos = Some(position);
            }
        }
        ParseAction::ConfirmBlockStart => {
            let start = state
                .potential_line_start_pos
                .or(state.potential_block_start_pos)
                .unwrap_or_else(|| position.saturating_sub(1));

            if !state.is_line_comment {
                let actual_start = if state.nest_level == 0 {
                    start
                } else {
                    state.start_pos
                };
                state = CommentTrackState {
                    nest_level: state.nest_level + 1,
                    start_pos: actual_start,
                    is_line_comment: false,
                    potential_block_start_pos: None,
                    potential_line_start_pos: None,
                };
            } else {
                state.potential_block_start_pos = None;
                state.potential_line_start_pos = None;
            }
        }
        ParseAction::PotentialBlockEnd => {}
        ParseAction::BlockCommentEnd => {
            if state.nest_level > 0 {
                state.nest_level -= 1;
                if state.nest_level == 0 {
                    matches.push(CommentMatch {
                        from: state.start_pos,
                        to: position + 1,
                    });
                    state = CommentTrackState::start();
                }
            }
        }
        ParseAction::ResetPotentialBlock => {
            if state.nest_level == 0 {
                state = CommentTrackState::start();
            } else {
                state.potential_block_start_pos = None;
                state.potential_line_start_pos = None;
            }
        }
    }
    Ok((state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl(input, state_transition, do_action)
}
