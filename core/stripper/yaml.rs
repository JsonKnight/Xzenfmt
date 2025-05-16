use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum YamlParseState {
    StartOfLine,
    Normal,
    SawHash,
    StringDbl,
    StringDblEsc,
    StringSgl,
    End,
}
impl Start for YamlParseState {
    fn start() -> Self {
        YamlParseState::StartOfLine
    }
}
impl End for YamlParseState {
    fn end() -> Self {
        YamlParseState::End
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

fn yaml_state_transition(
    from: YamlParseState,
    current_char: Option<char>,
) -> (YamlParseState, ParseAction) {
    match current_char {
        Some(c) => match from {
            YamlParseState::StartOfLine | YamlParseState::Normal => match c {
                '#' => (YamlParseState::SawHash, ParseAction::CommentStart),
                '"' => (YamlParseState::StringDbl, ParseAction::Nothing),
                '\'' => (YamlParseState::StringSgl, ParseAction::Nothing),
                ' ' | '\t' if from == YamlParseState::StartOfLine => {
                    (YamlParseState::StartOfLine, ParseAction::Nothing)
                }
                '\n' => (YamlParseState::StartOfLine, ParseAction::Nothing),
                _ => (YamlParseState::Normal, ParseAction::Nothing),
            },
            YamlParseState::SawHash => match c {
                '\n' => (YamlParseState::StartOfLine, ParseAction::CommentEnd),
                _ => (YamlParseState::SawHash, ParseAction::Nothing),
            },
            YamlParseState::StringDbl => match c {
                '"' => (YamlParseState::Normal, ParseAction::Nothing),
                '\\' => (YamlParseState::StringDblEsc, ParseAction::Nothing),
                _ => (YamlParseState::StringDbl, ParseAction::Nothing),
            },
            YamlParseState::StringDblEsc => (YamlParseState::StringDbl, ParseAction::Nothing),
            YamlParseState::StringSgl => match c {
                '\'' => (YamlParseState::Normal, ParseAction::Nothing),
                _ => (YamlParseState::StringSgl, ParseAction::Nothing),
            },
            YamlParseState::End => (YamlParseState::End, ParseAction::Nothing),
        },
        None => match from {
            YamlParseState::SawHash => (YamlParseState::End, ParseAction::CommentEnd),
            _ => (YamlParseState::End, ParseAction::Nothing),
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
    find_comments_impl(input, yaml_state_transition, do_action)
}
