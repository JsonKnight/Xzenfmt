use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    StartOfLine,
    Normal,
    SawHash,
    StringDblQuote,
    StringDblQuoteEscaped,
    StringSglQuote,
    End,
}
impl Start for ParseState {
    fn start() -> Self {
        ParseState::StartOfLine
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
    CommentStart,
    CommentEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommentTrackState {
    NotInComment,
    InLineComment(usize),
}
impl Start for CommentTrackState {
    fn start() -> Self {
        CommentTrackState::NotInComment
    }
}

fn state_transition(from: ParseState, current_char: Option<char>) -> (ParseState, ParseAction) {
    match current_char {
        Some(c) => match from {
            ParseState::StartOfLine | ParseState::Normal => match c {
                '#' => (ParseState::SawHash, ParseAction::CommentStart),
                '"' => (ParseState::StringDblQuote, ParseAction::Nothing),
                '\'' => (ParseState::StringSglQuote, ParseAction::Nothing),
                '\n' => (ParseState::StartOfLine, ParseAction::Nothing),
                ' ' | '\t' if from == ParseState::StartOfLine => {
                    (ParseState::StartOfLine, ParseAction::Nothing)
                }
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::SawHash => match c {
                '\n' => (ParseState::StartOfLine, ParseAction::CommentEnd),
                _ => (ParseState::SawHash, ParseAction::Nothing),
            },
            ParseState::StringDblQuote => match c {
                '"' => (ParseState::Normal, ParseAction::Nothing),
                '\\' => (ParseState::StringDblQuoteEscaped, ParseAction::Nothing),
                _ => (ParseState::StringDblQuote, ParseAction::Nothing),
            },
            ParseState::StringDblQuoteEscaped => (ParseState::StringDblQuote, ParseAction::Nothing),
            ParseState::StringSglQuote => match c {
                '\'' => (ParseState::Normal, ParseAction::Nothing),
                _ => (ParseState::StringSglQuote, ParseAction::Nothing),
            },
            ParseState::End => (ParseState::End, ParseAction::Nothing),
        },
        None => match from {
            ParseState::SawHash => (ParseState::End, ParseAction::CommentEnd),
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
        ParseAction::CommentStart => {
            if let CommentTrackState::NotInComment = comment_state {
                comment_state = CommentTrackState::InLineComment(position);
            }
        }
        ParseAction::CommentEnd => {
            if let CommentTrackState::InLineComment(from) = comment_state {
                matches.push(CommentMatch { from, to: position });
                comment_state = CommentTrackState::NotInComment;
            }
        }
    }
    Ok((comment_state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl(input, state_transition, do_action)
}
