use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    StartOfLine,
    Normal,
    SawHash,
    StringDbl,
    StringDblEsc,
    StringSgl,
    StringSglEsc,
    MaybeTripleDbl,
    MaybeTripleDbl2,
    InTripleDbl,
    InTripleDblSawD1,
    InTripleDblSawD2,
    MaybeTripleSgl,
    MaybeTripleSgl2,
    InTripleSgl,
    InTripleSglSawS1,
    InTripleSglSawS2,
    StringRawPrefix,
    StringRawSgl,
    StringRawDbl,
    MaybeRawTripleDbl,
    MaybeRawTripleDbl2,
    InRawTripleDbl,
    InRawTripleDblSawD1,
    InRawTripleDblSawD2,
    MaybeRawTripleSgl,
    MaybeRawTripleSgl2,
    InRawTripleSgl,
    InRawTripleSglSawS1,
    InRawTripleSglSawS2,
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
enum PyParseAction {
    Nothing,
    CommentStart,
    CommentEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PyCommentState {
    NotInComment,
    InComment(usize),
}
impl Start for PyCommentState {
    fn start() -> Self {
        PyCommentState::NotInComment
    }
}

fn state_transition(from: ParseState, current_char: Option<char>) -> (ParseState, PyParseAction) {
    match current_char {
        Some(c) => match from {
            ParseState::StartOfLine | ParseState::Normal => match c {
                '#' => (ParseState::SawHash, PyParseAction::CommentStart),
                '"' => (ParseState::MaybeTripleDbl, PyParseAction::Nothing),
                '\'' => (ParseState::MaybeTripleSgl, PyParseAction::Nothing),
                'r' | 'R' => (ParseState::StringRawPrefix, PyParseAction::Nothing),
                '\n' => (ParseState::StartOfLine, PyParseAction::Nothing),
                ' ' | '\t' if from == ParseState::StartOfLine => {
                    (ParseState::StartOfLine, PyParseAction::Nothing)
                }
                _ => (ParseState::Normal, PyParseAction::Nothing),
            },
            ParseState::SawHash => match c {
                '\n' => (ParseState::StartOfLine, PyParseAction::CommentEnd),
                _ => (ParseState::SawHash, PyParseAction::Nothing),
            },

            ParseState::StringDbl => match c {
                '"' => (ParseState::Normal, PyParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, PyParseAction::Nothing),
                _ => (ParseState::StringDbl, PyParseAction::Nothing),
            },
            ParseState::StringDblEsc => (ParseState::StringDbl, PyParseAction::Nothing),
            ParseState::StringSgl => match c {
                '\'' => (ParseState::Normal, PyParseAction::Nothing),
                '\\' => (ParseState::StringSglEsc, PyParseAction::Nothing),
                _ => (ParseState::StringSgl, PyParseAction::Nothing),
            },
            ParseState::StringSglEsc => (ParseState::StringSgl, PyParseAction::Nothing),

            ParseState::MaybeTripleDbl => match c {
                '"' => (ParseState::MaybeTripleDbl2, PyParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, PyParseAction::Nothing),
                _ => (ParseState::StringDbl, PyParseAction::Nothing),
            },
            ParseState::MaybeTripleDbl2 => match c {
                '"' => (ParseState::InTripleDbl, PyParseAction::Nothing),
                _ => (ParseState::Normal, PyParseAction::Nothing),
            },
            ParseState::InTripleDbl => match c {
                '"' => (ParseState::InTripleDblSawD1, PyParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, PyParseAction::Nothing),
                _ => (ParseState::InTripleDbl, PyParseAction::Nothing),
            },
            ParseState::InTripleDblSawD1 => match c {
                '"' => (ParseState::InTripleDblSawD2, PyParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, PyParseAction::Nothing),
                _ => (ParseState::InTripleDbl, PyParseAction::Nothing),
            },
            ParseState::InTripleDblSawD2 => match c {
                '"' => (ParseState::Normal, PyParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, PyParseAction::Nothing),
                _ => (ParseState::InTripleDbl, PyParseAction::Nothing),
            },

            ParseState::MaybeTripleSgl => match c {
                '\'' => (ParseState::MaybeTripleSgl2, PyParseAction::Nothing),
                '\\' => (ParseState::StringSglEsc, PyParseAction::Nothing),
                _ => (ParseState::StringSgl, PyParseAction::Nothing),
            },
            ParseState::MaybeTripleSgl2 => match c {
                '\'' => (ParseState::InTripleSgl, PyParseAction::Nothing),
                _ => (ParseState::Normal, PyParseAction::Nothing),
            },
            ParseState::InTripleSgl => match c {
                '\'' => (ParseState::InTripleSglSawS1, PyParseAction::Nothing),
                '\\' => (ParseState::StringSglEsc, PyParseAction::Nothing),
                _ => (ParseState::InTripleSgl, PyParseAction::Nothing),
            },
            ParseState::InTripleSglSawS1 => match c {
                '\'' => (ParseState::InTripleSglSawS2, PyParseAction::Nothing),
                '\\' => (ParseState::StringSglEsc, PyParseAction::Nothing),
                _ => (ParseState::InTripleSgl, PyParseAction::Nothing),
            },
            ParseState::InTripleSglSawS2 => match c {
                '\'' => (ParseState::Normal, PyParseAction::Nothing),
                '\\' => (ParseState::StringSglEsc, PyParseAction::Nothing),
                _ => (ParseState::InTripleSgl, PyParseAction::Nothing),
            },

            ParseState::StringRawPrefix => match c {
                '"' => (ParseState::MaybeRawTripleDbl, PyParseAction::Nothing),
                '\'' => (ParseState::MaybeRawTripleSgl, PyParseAction::Nothing),
                _ => (ParseState::Normal, PyParseAction::Nothing),
            },
            ParseState::StringRawSgl => match c {
                '\'' => (ParseState::Normal, PyParseAction::Nothing),
                _ => (ParseState::StringRawSgl, PyParseAction::Nothing),
            },
            ParseState::StringRawDbl => match c {
                '"' => (ParseState::Normal, PyParseAction::Nothing),
                _ => (ParseState::StringRawDbl, PyParseAction::Nothing),
            },
            ParseState::MaybeRawTripleDbl => match c {
                '"' => (ParseState::MaybeRawTripleDbl2, PyParseAction::Nothing),
                _ => (ParseState::StringRawDbl, PyParseAction::Nothing),
            },
            ParseState::MaybeRawTripleDbl2 => match c {
                '"' => (ParseState::InRawTripleDbl, PyParseAction::Nothing),
                _ => (ParseState::StringRawDbl, PyParseAction::Nothing),
            },
            ParseState::InRawTripleDbl => match c {
                '"' => (ParseState::InRawTripleDblSawD1, PyParseAction::Nothing),
                _ => (ParseState::InRawTripleDbl, PyParseAction::Nothing),
            },
            ParseState::InRawTripleDblSawD1 => match c {
                '"' => (ParseState::InRawTripleDblSawD2, PyParseAction::Nothing),
                _ => (ParseState::InRawTripleDbl, PyParseAction::Nothing),
            },
            ParseState::InRawTripleDblSawD2 => match c {
                '"' => (ParseState::Normal, PyParseAction::Nothing),
                _ => (ParseState::InRawTripleDbl, PyParseAction::Nothing),
            },
            ParseState::MaybeRawTripleSgl => match c {
                '\'' => (ParseState::MaybeRawTripleSgl2, PyParseAction::Nothing),
                _ => (ParseState::StringRawSgl, PyParseAction::Nothing),
            },
            ParseState::MaybeRawTripleSgl2 => match c {
                '\'' => (ParseState::InRawTripleSgl, PyParseAction::Nothing),
                _ => (ParseState::StringRawSgl, PyParseAction::Nothing),
            },
            ParseState::InRawTripleSgl => match c {
                '\'' => (ParseState::InRawTripleSglSawS1, PyParseAction::Nothing),
                _ => (ParseState::InRawTripleSgl, PyParseAction::Nothing),
            },
            ParseState::InRawTripleSglSawS1 => match c {
                '\'' => (ParseState::InRawTripleSglSawS2, PyParseAction::Nothing),
                _ => (ParseState::InRawTripleSgl, PyParseAction::Nothing),
            },
            ParseState::InRawTripleSglSawS2 => match c {
                '\'' => (ParseState::Normal, PyParseAction::Nothing),
                _ => (ParseState::InRawTripleSgl, PyParseAction::Nothing),
            },

            ParseState::End => (ParseState::End, PyParseAction::Nothing),
        },
        None => match from {
            ParseState::SawHash => (ParseState::End, PyParseAction::CommentEnd),
            _ => (ParseState::End, PyParseAction::Nothing),
        },
    }
}

fn do_action(
    action: PyParseAction,
    mut comment_state: PyCommentState,
    position: usize,
    mut matches: Vec<CommentMatch>,
) -> Result<(PyCommentState, Vec<CommentMatch>), StripError> {
    match action {
        PyParseAction::Nothing => {}
        PyParseAction::CommentStart => {
            if let PyCommentState::NotInComment = comment_state {
                comment_state = PyCommentState::InComment(position);
            }
        }
        PyParseAction::CommentEnd => {
            if let PyCommentState::InComment(from) = comment_state {
                matches.push(CommentMatch { from, to: position });
                comment_state = PyCommentState::NotInComment;
            }
        }
    }
    Ok((comment_state, matches))
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl(input, state_transition, do_action)
}
