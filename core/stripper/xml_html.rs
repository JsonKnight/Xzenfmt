use super::common::{CommentMatch, End, Start, StripError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Normal,
    SawOpenBracket,
    SawOpenBracketBang,
    SawOpenBracketBangDash,
    InComment,
    InCommentSawDash1,
    InCommentSawDash2,
    StringDbl,
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
    MaybeCommentStart1,
    MaybeCommentStart2,
    MaybeCommentStart3,
    CommentStart,
    CommentEnd,
    ResetPotential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommentTrackState {
    NotInComment,
    MaybeComment(usize),
    InComment(usize),
}
impl Start for CommentTrackState {
    fn start() -> Self {
        CommentTrackState::NotInComment
    }
}

fn state_transition(from: ParseState, current_char: Option<char>) -> (ParseState, ParseAction) {
    match current_char {
        Some(c) => match from {
            ParseState::Normal => match c {
                '<' => (ParseState::SawOpenBracket, ParseAction::MaybeCommentStart1),
                '"' => (ParseState::StringDbl, ParseAction::Nothing),
                '\'' => (ParseState::StringSgl, ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::SawOpenBracket => match c {
                '!' => (
                    ParseState::SawOpenBracketBang,
                    ParseAction::MaybeCommentStart2,
                ),
                '"' => (ParseState::StringDbl, ParseAction::ResetPotential),
                '\'' => (ParseState::StringSgl, ParseAction::ResetPotential),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::SawOpenBracketBang => match c {
                '-' => (
                    ParseState::SawOpenBracketBangDash,
                    ParseAction::MaybeCommentStart3,
                ),
                '"' => (ParseState::StringDbl, ParseAction::ResetPotential),
                '\'' => (ParseState::StringSgl, ParseAction::ResetPotential),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::SawOpenBracketBangDash => match c {
                '-' => (ParseState::InComment, ParseAction::CommentStart),
                '"' => (ParseState::StringDbl, ParseAction::ResetPotential),
                '\'' => (ParseState::StringSgl, ParseAction::ResetPotential),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::InComment => match c {
                '-' => (ParseState::InCommentSawDash1, ParseAction::Nothing),
                _ => (ParseState::InComment, ParseAction::Nothing),
            },
            ParseState::InCommentSawDash1 => match c {
                '-' => (ParseState::InCommentSawDash2, ParseAction::Nothing),
                _ => (ParseState::InComment, ParseAction::Nothing),
            },
            ParseState::InCommentSawDash2 => match c {
                '>' => (ParseState::Normal, ParseAction::CommentEnd),
                '-' => (ParseState::InCommentSawDash2, ParseAction::Nothing),
                _ => (ParseState::InComment, ParseAction::Nothing),
            },
            ParseState::StringDbl => match c {
                '"' => (ParseState::Normal, ParseAction::Nothing),
                _ => (ParseState::StringDbl, ParseAction::Nothing),
            },
            ParseState::StringSgl => match c {
                '\'' => (ParseState::Normal, ParseAction::Nothing),
                _ => (ParseState::StringSgl, ParseAction::Nothing),
            },
            ParseState::End => (ParseState::End, ParseAction::Nothing),
        },
        None => match from {
            ParseState::SawOpenBracket
            | ParseState::SawOpenBracketBang
            | ParseState::SawOpenBracketBangDash
            | ParseState::InComment
            | ParseState::InCommentSawDash1
            | ParseState::InCommentSawDash2 => (ParseState::End, ParseAction::ResetPotential),
            _ => (ParseState::End, ParseAction::Nothing),
        },
    }
}

fn do_action(
    action: ParseAction,
    mut comment_state: CommentTrackState,
    position: usize,
    mut matches: Vec<CommentMatch>,
) -> Result<(CommentTrackState, Vec<CommentMatch>), StripError> {
    match action {
        ParseAction::Nothing => {}
        ParseAction::MaybeCommentStart1 => {
            if let CommentTrackState::NotInComment = comment_state {
                comment_state = CommentTrackState::MaybeComment(position);
            }
        }
        ParseAction::MaybeCommentStart2 | ParseAction::MaybeCommentStart3 => {
            if let CommentTrackState::MaybeComment(_) = comment_state {
            } else {
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::CommentStart => {
            if let CommentTrackState::MaybeComment(from) = comment_state {
                comment_state = CommentTrackState::InComment(from);
            } else {
                return Err("XML Stripper Error: Invalid state transition to CommentStart");
            }
        }
        ParseAction::CommentEnd => {
            if let CommentTrackState::InComment(from) = comment_state {
                matches.push(CommentMatch {
                    from,
                    to: position + 1,
                });
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::ResetPotential => {
            comment_state = CommentTrackState::NotInComment;
        }
    }
    Ok((comment_state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    super::common::find_comments_impl(input, state_transition, do_action)
}
