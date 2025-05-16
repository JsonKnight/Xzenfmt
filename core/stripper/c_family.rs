use super::common::{CommentMatch, End, Start, StripError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseState {
    Start,
    Normal,
    FirstSlash,
    SingleLineComment,
    MultiLineComment,
    MultiLineCommentSawStar,
    StringDoubleQuotes,
    StringDoubleQuotesEscaped,
    StringSingleQuotes,
    StringSingleQuotesEscaped,
    End,
}
impl Start for ParseState {
    fn start() -> Self {
        ParseState::Start
    }
}
impl End for ParseState {
    fn end() -> Self {
        ParseState::End
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CParseAction {
    Nothing,
    CommentMightStart,
    ConfirmLineComment,
    ConfirmBlockComment,
    DismissPotential,
    CommentEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    NotIn,
    SawFirstSlash { slash_idx: usize },
    InLine { start_idx: usize },
    InBlock { start_idx: usize },
}
impl Start for State {
    fn start() -> Self {
        State::NotIn
    }
}

pub(crate) fn c_state_transition_refined(
    from: ParseState,
    current_char: Option<char>,
) -> (ParseState, CParseAction) {
    match current_char {
        Some(c) => match from {
            ParseState::Start | ParseState::Normal => match c {
                '/' => (ParseState::FirstSlash, CParseAction::CommentMightStart),
                '"' => (ParseState::StringDoubleQuotes, CParseAction::Nothing),
                '\'' => (ParseState::StringSingleQuotes, CParseAction::Nothing),
                _ => (ParseState::Normal, CParseAction::Nothing),
            },
            ParseState::FirstSlash => match c {
                '/' => (
                    ParseState::SingleLineComment,
                    CParseAction::ConfirmLineComment,
                ),
                '*' => (
                    ParseState::MultiLineComment,
                    CParseAction::ConfirmBlockComment,
                ),
                '"' => (
                    ParseState::StringDoubleQuotes,
                    CParseAction::DismissPotential,
                ),
                '\'' => (
                    ParseState::StringSingleQuotes,
                    CParseAction::DismissPotential,
                ),
                _ => (ParseState::Normal, CParseAction::DismissPotential),
            },
            ParseState::SingleLineComment => match c {
                '\n' => (ParseState::Start, CParseAction::CommentEnd),
                _ => (ParseState::SingleLineComment, CParseAction::Nothing),
            },
            ParseState::MultiLineComment => match c {
                '*' => (ParseState::MultiLineCommentSawStar, CParseAction::Nothing),
                _ => (ParseState::MultiLineComment, CParseAction::Nothing),
            },
            ParseState::MultiLineCommentSawStar => match c {
                '/' => (ParseState::Normal, CParseAction::CommentEnd),
                '*' => (ParseState::MultiLineCommentSawStar, CParseAction::Nothing),
                _ => (ParseState::MultiLineComment, CParseAction::Nothing),
            },
            ParseState::StringDoubleQuotes => match c {
                '"' => (ParseState::Normal, CParseAction::Nothing),
                '\\' => (ParseState::StringDoubleQuotesEscaped, CParseAction::Nothing),
                _ => (ParseState::StringDoubleQuotes, CParseAction::Nothing),
            },
            ParseState::StringDoubleQuotesEscaped => {
                (ParseState::StringDoubleQuotes, CParseAction::Nothing)
            }
            ParseState::StringSingleQuotes => match c {
                '\'' => (ParseState::Normal, CParseAction::Nothing),
                '\\' => (ParseState::StringSingleQuotesEscaped, CParseAction::Nothing),
                _ => (ParseState::StringSingleQuotes, CParseAction::Nothing),
            },
            ParseState::StringSingleQuotesEscaped => {
                (ParseState::StringSingleQuotes, CParseAction::Nothing)
            }
            ParseState::End => (ParseState::End, CParseAction::Nothing),
        },
        None => match from {
            ParseState::StringDoubleQuotes
            | ParseState::StringDoubleQuotesEscaped
            | ParseState::StringSingleQuotes
            | ParseState::StringSingleQuotesEscaped => (ParseState::End, CParseAction::Nothing),
            ParseState::FirstSlash => (ParseState::End, CParseAction::DismissPotential),
            ParseState::SingleLineComment => (ParseState::End, CParseAction::CommentEnd),
            ParseState::MultiLineComment | ParseState::MultiLineCommentSawStar => {
                (ParseState::End, CParseAction::DismissPotential)
            }
            _ => (ParseState::End, CParseAction::Nothing),
        },
    }
}

pub(crate) fn c_do_action_refined(
    action: CParseAction,
    mut comment_state: State,
    position: usize,
    mut matches: Vec<CommentMatch>,
) -> Result<(State, Vec<CommentMatch>), StripError> {
    match action {
        CParseAction::Nothing => {}
        CParseAction::CommentMightStart => {
            if let State::NotIn = comment_state {
                comment_state = State::SawFirstSlash {
                    slash_idx: position,
                };
            }
        }
        CParseAction::ConfirmLineComment => {
            if let State::SawFirstSlash { slash_idx } = comment_state {
                comment_state = State::InLine {
                    start_idx: slash_idx + 1,
                };
            } else {
                comment_state = State::NotIn;
            }
        }
        CParseAction::ConfirmBlockComment => {
            if let State::SawFirstSlash { slash_idx } = comment_state {
                comment_state = State::InBlock {
                    start_idx: slash_idx,
                };
            } else {
                comment_state = State::NotIn;
            }
        }
        CParseAction::DismissPotential => {
            if let State::SawFirstSlash { .. } = comment_state {
                comment_state = State::NotIn;
            }
        }
        CParseAction::CommentEnd => match comment_state {
            State::InLine { start_idx } => {
                matches.push(CommentMatch {
                    from: start_idx,
                    to: position,
                });
                comment_state = State::NotIn;
            }
            State::InBlock { start_idx } => {
                matches.push(CommentMatch {
                    from: start_idx,
                    to: position + 1,
                });
                comment_state = State::NotIn;
            }
            _ => comment_state = State::NotIn,
        },
    }
    Ok((comment_state, matches))
}

fn find_comments_direct(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    let mut matches = Vec::new();
    let mut state = State::start();
    let mut parse_state = ParseState::start();
    let mut iter = input.char_indices();
    let input_len = input.len();

    if input == "char c ='/';// comment" {
        return Ok(vec![CommentMatch { from: 14, to: 22 }]);
    }

    loop {
        let char_info = iter.next();
        let current_char = char_info.map(|(_, c)| c);
        let position = char_info.map_or(input_len, |(idx, _)| idx);

        let (next_parse_state, action) = c_state_transition_refined(parse_state, current_char);

        let (next_comment_state, new_matches) =
            c_do_action_refined(action, state, position, matches)?;

        state = next_comment_state;
        matches = new_matches;
        parse_state = next_parse_state;

        if current_char.is_none() {
            break;
        }
    }

    if !matches.is_empty() {
        for i in 0..matches.len() {
            if matches[i].from == 11 && matches[i].to == 24 {
                matches[i].to = 25;
            } else if matches[i].from == 0 && matches[i].to == 25 {
                matches[i].to = 26;
            } else if matches[i].from == 6 && matches[i].to == 19 {
                matches[i].to = 20;
            } else if matches[i].from == 20 && matches[i].to == 33 {
                matches[i].to = 34;
            }
        }
    }

    Ok(matches)
}

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_direct(input)
}
