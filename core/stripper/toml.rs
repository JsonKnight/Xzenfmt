use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TomlParseState {
    StartOfLine,
    Normal,
    SawHash,
    StringDbl,
    StringDblEsc,
    StringSgl,
    End,
}
impl Start for TomlParseState {
    fn start() -> Self {
        TomlParseState::StartOfLine
    }
}
impl End for TomlParseState {
    fn end() -> Self {
        TomlParseState::End
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
    InComment(usize),
}
impl Start for CommentTrackState {
    fn start() -> Self {
        CommentTrackState::NotInComment
    }
}

fn toml_state_transition(
    from: TomlParseState,
    current_char: Option<char>,
) -> (TomlParseState, ParseAction) {
    match current_char {
        Some(c) => match from {
            TomlParseState::StartOfLine | TomlParseState::Normal => match c {
                '#' => (TomlParseState::SawHash, ParseAction::CommentStart),
                '"' => (TomlParseState::StringDbl, ParseAction::Nothing),
                '\'' => (TomlParseState::StringSgl, ParseAction::Nothing),
                ' ' | '\t' if from == TomlParseState::StartOfLine => {
                    (TomlParseState::StartOfLine, ParseAction::Nothing)
                }
                '\n' => (TomlParseState::StartOfLine, ParseAction::Nothing),
                _ => (TomlParseState::Normal, ParseAction::Nothing),
            },
            TomlParseState::SawHash => match c {
                '\n' => (TomlParseState::StartOfLine, ParseAction::CommentEnd),
                _ => (TomlParseState::SawHash, ParseAction::Nothing),
            },
            TomlParseState::StringDbl => match c {
                '"' => (TomlParseState::Normal, ParseAction::Nothing),
                '\\' => (TomlParseState::StringDblEsc, ParseAction::Nothing),
                _ => (TomlParseState::StringDbl, ParseAction::Nothing),
            },
            TomlParseState::StringDblEsc => (TomlParseState::StringDbl, ParseAction::Nothing),
            TomlParseState::StringSgl => match c {
                '\'' => (TomlParseState::Normal, ParseAction::Nothing),
                _ => (TomlParseState::StringSgl, ParseAction::Nothing),
            },
            TomlParseState::End => (TomlParseState::End, ParseAction::Nothing),
        },
        None => match from {
            TomlParseState::SawHash => (TomlParseState::End, ParseAction::CommentEnd),
            _ => (TomlParseState::End, ParseAction::Nothing),
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
                comment_state = CommentTrackState::InComment(position);
            }
        }
        ParseAction::CommentEnd => {
            if let CommentTrackState::InComment(from) = comment_state {
                matches.push(CommentMatch { from, to: position });
                comment_state = CommentTrackState::NotInComment;
            }
        }
    }
    Ok((comment_state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl(input, toml_state_transition, do_action)
}
