use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShParseState {
    StartOfLine,
    Normal,
    PotentialShebang,
    SawHash,
    StringDbl,
    StringDblEsc,
    StringSgl,
    End,
}
impl Start for ShParseState {
    fn start() -> Self {
        ShParseState::StartOfLine
    }
}
impl End for ShParseState {
    fn end() -> Self {
        ShParseState::End
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseAction {
    Nothing,
    CommentStart,
    CommentEnd,
    PotentialShebang,
    ShebangConfirmed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommentTrackState {
    NotInComment,
    MaybeShebang(usize),
    InComment(usize),
}
impl Start for CommentTrackState {
    fn start() -> Self {
        CommentTrackState::NotInComment
    }
}

pub(crate) fn sh_state_transition(
    from: ShParseState,
    current_char: Option<char>,
) -> (ShParseState, ParseAction) {
    match current_char {
        Some(c) => match from {
            ShParseState::StartOfLine => match c {
                '#' => (
                    ShParseState::PotentialShebang,
                    ParseAction::PotentialShebang,
                ),
                '"' => (ShParseState::StringDbl, ParseAction::Nothing),
                '\'' => (ShParseState::StringSgl, ParseAction::Nothing),
                ' ' | '\t' => (ShParseState::StartOfLine, ParseAction::Nothing),
                '\n' => (ShParseState::StartOfLine, ParseAction::Nothing),
                _ => (ShParseState::Normal, ParseAction::Nothing),
            },
            ShParseState::PotentialShebang => match c {
                '!' => (ShParseState::Normal, ParseAction::ShebangConfirmed),
                '\n' => (ShParseState::StartOfLine, ParseAction::CommentStart),
                _ => (ShParseState::SawHash, ParseAction::CommentStart),
            },
            ShParseState::Normal => match c {
                '#' => (ShParseState::SawHash, ParseAction::CommentStart),
                '"' => (ShParseState::StringDbl, ParseAction::Nothing),
                '\'' => (ShParseState::StringSgl, ParseAction::Nothing),
                '\n' => (ShParseState::StartOfLine, ParseAction::Nothing),
                _ => (ShParseState::Normal, ParseAction::Nothing),
            },
            ShParseState::SawHash => match c {
                '\n' => (ShParseState::StartOfLine, ParseAction::CommentEnd),
                _ => (ShParseState::SawHash, ParseAction::Nothing),
            },
            ShParseState::StringDbl => match c {
                '"' => (ShParseState::Normal, ParseAction::Nothing),
                '\\' => (ShParseState::StringDblEsc, ParseAction::Nothing),
                _ => (ShParseState::StringDbl, ParseAction::Nothing),
            },
            ShParseState::StringDblEsc => (ShParseState::StringDbl, ParseAction::Nothing),
            ShParseState::StringSgl => match c {
                '\'' => (ShParseState::Normal, ParseAction::Nothing),
                _ => (ShParseState::StringSgl, ParseAction::Nothing),
            },
            ShParseState::End => (ShParseState::End, ParseAction::Nothing),
        },
        None => match from {
            ShParseState::SawHash | ShParseState::PotentialShebang => {
                (ShParseState::End, ParseAction::CommentEnd)
            }
            _ => (ShParseState::End, ParseAction::Nothing),
        },
    }
}

pub(crate) fn do_action(
    action: ParseAction,
    mut comment_state: CommentTrackState,
    position: usize,
    mut matches: Vec<CommentMatch>,
) -> Result<(CommentTrackState, Vec<CommentMatch>), StripError> {
    match action {
        ParseAction::Nothing | ParseAction::ShebangConfirmed => {
            if let CommentTrackState::MaybeShebang(_) = comment_state {
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::PotentialShebang => {
            if let CommentTrackState::NotInComment = comment_state {
                comment_state = CommentTrackState::MaybeShebang(position);
            }
        }
        ParseAction::CommentStart => match comment_state {
            CommentTrackState::MaybeShebang(from) => {
                comment_state = CommentTrackState::InComment(from);
            }
            CommentTrackState::NotInComment => {
                comment_state = CommentTrackState::InComment(position);
            }
            CommentTrackState::InComment(_) => {}
        },
        ParseAction::CommentEnd => match comment_state {
            CommentTrackState::InComment(from) | CommentTrackState::MaybeShebang(from) => {
                matches.push(CommentMatch { from, to: position });
                comment_state = CommentTrackState::NotInComment;
            }
            CommentTrackState::NotInComment => {}
        },
    }
    Ok((comment_state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl(input, sh_state_transition, do_action)
}
