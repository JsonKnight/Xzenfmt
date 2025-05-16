use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Normal,
    SawOpenBrace,
    InComment,
    InCommentSawHash,
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
    MaybeCommentStart,
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
                '{' => (ParseState::SawOpenBrace, ParseAction::MaybeCommentStart),
                '"' => (ParseState::StringDbl, ParseAction::Nothing),
                '\'' => (ParseState::StringSgl, ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::SawOpenBrace => match c {
                '#' => (ParseState::InComment, ParseAction::CommentStart),
                '{' | '%' => (ParseState::Normal, ParseAction::ResetPotential),
                '"' => (ParseState::StringDbl, ParseAction::ResetPotential),
                '\'' => (ParseState::StringSgl, ParseAction::ResetPotential),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::InComment => match c {
                '#' => (ParseState::InCommentSawHash, ParseAction::Nothing),
                _ => (ParseState::InComment, ParseAction::Nothing),
            },
            ParseState::InCommentSawHash => match c {
                '}' => (ParseState::Normal, ParseAction::CommentEnd),
                '#' => (ParseState::InCommentSawHash, ParseAction::Nothing),
                _ => (ParseState::InComment, ParseAction::Nothing),
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
            ParseState::SawOpenBrace | ParseState::InComment | ParseState::InCommentSawHash => {
                (ParseState::End, ParseAction::ResetPotential)
            }
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
        ParseAction::MaybeCommentStart => {
            if let CommentTrackState::NotInComment = comment_state {
                comment_state = CommentTrackState::MaybeComment(position);
            }
        }
        ParseAction::CommentStart => match comment_state {
            CommentTrackState::MaybeComment(from) => {
                comment_state = CommentTrackState::InComment(from);
            }
            _ => comment_state = CommentTrackState::NotInComment,
        },
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
            if let CommentTrackState::MaybeComment(_) | CommentTrackState::InComment(_) =
                comment_state
            {
                comment_state = CommentTrackState::NotInComment;
            }
        }
    }
    Ok((comment_state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl(input, state_transition, do_action)
}
