use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Normal,
    SawDash1,
    LineComment,
    MaybeBlockCommentOpen1,
    InBlockComment,
    InBlockCommentSawDash,
    StringDbl,
    StringDblEsc,
    CharSgl,
    CharSglEsc,
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
    PotentialLineComment,
    LineCommentStart,
    LineCommentEnd,
    MaybeBlockStart,
    ConfirmBlockStart,
    MaybeBlockEnd,
    BlockCommentEnd,
    ResetPotential,
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
                '-' => (ParseState::SawDash1, ParseAction::PotentialLineComment),
                '{' => (
                    ParseState::MaybeBlockCommentOpen1,
                    ParseAction::MaybeBlockStart,
                ),
                '"' => (ParseState::StringDbl, ParseAction::Nothing),
                '\'' => (ParseState::CharSgl, ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::SawDash1 => match c {
                '-' => (ParseState::LineComment, ParseAction::LineCommentStart),
                '{' => (
                    ParseState::MaybeBlockCommentOpen1,
                    ParseAction::MaybeBlockStart,
                ),
                '"' => (ParseState::StringDbl, ParseAction::ResetPotential),
                '\'' => (ParseState::CharSgl, ParseAction::ResetPotential),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::LineComment => match c {
                '\n' => (ParseState::Normal, ParseAction::LineCommentEnd),
                _ => (ParseState::LineComment, ParseAction::Nothing),
            },
            ParseState::MaybeBlockCommentOpen1 => match c {
                '-' => (ParseState::InBlockComment, ParseAction::ConfirmBlockStart),
                '{' => (
                    ParseState::MaybeBlockCommentOpen1,
                    ParseAction::MaybeBlockStart,
                ),
                '"' => (ParseState::StringDbl, ParseAction::ResetPotential),
                '\'' => (ParseState::CharSgl, ParseAction::ResetPotential),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::InBlockComment => match c {
                '-' => (
                    ParseState::InBlockCommentSawDash,
                    ParseAction::MaybeBlockEnd,
                ),
                '{' => (
                    ParseState::MaybeBlockCommentOpen1,
                    ParseAction::MaybeBlockStart,
                ),
                _ => (ParseState::InBlockComment, ParseAction::Nothing),
            },
            ParseState::InBlockCommentSawDash => match c {
                '}' => (ParseState::Normal, ParseAction::BlockCommentEnd),
                '-' => (
                    ParseState::InBlockCommentSawDash,
                    ParseAction::MaybeBlockEnd,
                ),
                '{' => (
                    ParseState::MaybeBlockCommentOpen1,
                    ParseAction::MaybeBlockStart,
                ),
                _ => (ParseState::InBlockComment, ParseAction::Nothing),
            },
            ParseState::StringDbl => match c {
                '"' => (ParseState::Normal, ParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, ParseAction::Nothing),
                _ => (ParseState::StringDbl, ParseAction::Nothing),
            },
            ParseState::StringDblEsc => (ParseState::StringDbl, ParseAction::Nothing),
            ParseState::CharSgl => match c {
                '\'' => (ParseState::Normal, ParseAction::Nothing),
                '\\' => (ParseState::CharSglEsc, ParseAction::Nothing),
                _ => (ParseState::CharSgl, ParseAction::Nothing),
            },
            ParseState::CharSglEsc => (ParseState::CharSgl, ParseAction::Nothing),
            ParseState::End => (ParseState::End, ParseAction::Nothing),
        },
        None => match from {
            ParseState::SawDash1 => (ParseState::End, ParseAction::ResetPotential),
            ParseState::LineComment => (ParseState::End, ParseAction::LineCommentEnd),
            ParseState::MaybeBlockCommentOpen1
            | ParseState::InBlockComment
            | ParseState::InBlockCommentSawDash => (ParseState::End, ParseAction::ResetPotential),
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
        ParseAction::Nothing => {}
        ParseAction::PotentialLineComment => {
            if state.nest_level == 0 && !state.is_line_comment {
                state.potential_line_start_pos = Some(position);
            }
            state.potential_block_start_pos = None;
        }
        ParseAction::LineCommentStart => {
            if let Some(from) = state.potential_line_start_pos {
                if state.nest_level == 0 && !state.is_line_comment {
                    state = CommentTrackState {
                        nest_level: 0,
                        start_pos: from,
                        is_line_comment: true,
                        potential_block_start_pos: None,
                        potential_line_start_pos: None,
                    };
                }
            } else {
                if state.nest_level == 0 && !state.is_line_comment {
                    state = CommentTrackState {
                        nest_level: 0,
                        start_pos: position.saturating_sub(1),
                        is_line_comment: true,
                        potential_block_start_pos: None,
                        potential_line_start_pos: None,
                    };
                }
            }
        }
        ParseAction::LineCommentEnd => {
            if state.is_line_comment && state.nest_level == 0 {
                matches.push(CommentMatch {
                    from: state.start_pos,
                    to: position,
                });
                state = CommentTrackState::start();
            } else if state.potential_line_start_pos.is_some() {
                state = CommentTrackState::start();
            }
        }
        ParseAction::MaybeBlockStart => {
            if state.nest_level == 0 && !state.is_line_comment {
                state.potential_block_start_pos = Some(position);
            } else if state.nest_level > 0 {
                state.potential_block_start_pos = Some(position);
            }
        }
        ParseAction::ConfirmBlockStart => {
            if let Some(start) = state.potential_block_start_pos {
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
            } else {
                state = CommentTrackState::start();
            }
        }
        ParseAction::MaybeBlockEnd => {}
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
        ParseAction::ResetPotential => {
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
