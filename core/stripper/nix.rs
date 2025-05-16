use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    StartOfLine,
    Normal,
    LineComment,
    SawSlash,
    BlockComment,
    BlockCommentSawStar,
    StringDbl,
    StringDblEsc,
    MaybeIndentedStringStart1,
    MaybeIndentedStringStart2,
    InIndentedString,
    InIndentedStringSawApos1,
    InIndentedStringSawApos2,
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
    LineCommentStart,
    LineCommentEnd,
    BlockCommentStart,
    BlockCommentEnd,
    PotentialBlockStart,
    DismissPotentialBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommentTrackState {
    NotInComment,
    MaybeBlock(usize),
    InLineComment(usize),
    InBlockComment(usize),
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
                '#' => (ParseState::LineComment, ParseAction::LineCommentStart),
                '/' => (ParseState::SawSlash, ParseAction::PotentialBlockStart),
                '"' => (ParseState::StringDbl, ParseAction::Nothing),
                '\'' => (ParseState::MaybeIndentedStringStart1, ParseAction::Nothing),
                '\n' => (ParseState::StartOfLine, ParseAction::Nothing),
                ' ' | '\t' if from == ParseState::StartOfLine => {
                    (ParseState::StartOfLine, ParseAction::Nothing)
                }
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::LineComment => match c {
                '\n' => (ParseState::StartOfLine, ParseAction::LineCommentEnd),
                _ => (ParseState::LineComment, ParseAction::Nothing),
            },
            ParseState::SawSlash => match c {
                '*' => (ParseState::BlockComment, ParseAction::BlockCommentStart),
                '#' => (ParseState::LineComment, ParseAction::DismissPotentialBlock),
                '/' => (ParseState::SawSlash, ParseAction::DismissPotentialBlock),
                '"' => (ParseState::StringDbl, ParseAction::DismissPotentialBlock),
                '\'' => (
                    ParseState::MaybeIndentedStringStart1,
                    ParseAction::DismissPotentialBlock,
                ),
                '\n' => (ParseState::StartOfLine, ParseAction::DismissPotentialBlock),
                _ => (ParseState::Normal, ParseAction::DismissPotentialBlock),
            },
            ParseState::BlockComment => match c {
                '*' => (ParseState::BlockCommentSawStar, ParseAction::Nothing),
                _ => (ParseState::BlockComment, ParseAction::Nothing),
            },
            ParseState::BlockCommentSawStar => match c {
                '/' => (ParseState::Normal, ParseAction::BlockCommentEnd),
                '*' => (ParseState::BlockCommentSawStar, ParseAction::Nothing),
                _ => (ParseState::BlockComment, ParseAction::Nothing),
            },
            ParseState::StringDbl => match c {
                '"' => (ParseState::Normal, ParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, ParseAction::Nothing),
                _ => (ParseState::StringDbl, ParseAction::Nothing),
            },
            ParseState::StringDblEsc => (ParseState::StringDbl, ParseAction::Nothing),

            ParseState::MaybeIndentedStringStart1 => match c {
                '\'' => (ParseState::MaybeIndentedStringStart2, ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::MaybeIndentedStringStart2 => match c {
                '\'' => (ParseState::InIndentedString, ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::InIndentedString => match c {
                '\'' => (ParseState::InIndentedStringSawApos1, ParseAction::Nothing),
                _ => (ParseState::InIndentedString, ParseAction::Nothing),
            },
            ParseState::InIndentedStringSawApos1 => match c {
                '\'' => (ParseState::InIndentedStringSawApos2, ParseAction::Nothing),
                _ => (ParseState::InIndentedString, ParseAction::Nothing),
            },
            ParseState::InIndentedStringSawApos2 => match c {
                '\'' => (ParseState::Normal, ParseAction::Nothing),
                _ => (ParseState::InIndentedString, ParseAction::Nothing),
            },

            ParseState::End => (ParseState::End, ParseAction::Nothing),
        },
        None => match from {
            ParseState::LineComment => (ParseState::End, ParseAction::LineCommentEnd),
            ParseState::SawSlash => (ParseState::End, ParseAction::DismissPotentialBlock),
            ParseState::BlockComment | ParseState::BlockCommentSawStar => {
                (ParseState::End, ParseAction::DismissPotentialBlock)
            }
            ParseState::MaybeIndentedStringStart1
            | ParseState::MaybeIndentedStringStart2
            | ParseState::InIndentedString
            | ParseState::InIndentedStringSawApos1
            | ParseState::InIndentedStringSawApos2 => (ParseState::End, ParseAction::Nothing),
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
        ParseAction::LineCommentStart => {
            if let CommentTrackState::NotInComment | CommentTrackState::MaybeBlock(_) =
                comment_state
            {
                comment_state = CommentTrackState::InLineComment(position);
            }
        }
        ParseAction::LineCommentEnd => {
            if let CommentTrackState::InLineComment(from) = comment_state {
                matches.push(CommentMatch { from, to: position });
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::PotentialBlockStart => {
            if let CommentTrackState::NotInComment = comment_state {
                comment_state = CommentTrackState::MaybeBlock(position);
            }
        }
        ParseAction::BlockCommentStart => match comment_state {
            CommentTrackState::MaybeBlock(from) => {
                comment_state = CommentTrackState::InBlockComment(from);
            }
            _ => {
                comment_state = CommentTrackState::NotInComment;
            }
        },
        ParseAction::BlockCommentEnd => {
            if let CommentTrackState::InBlockComment(from) = comment_state {
                matches.push(CommentMatch {
                    from,
                    to: position + 1,
                });
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::DismissPotentialBlock => {
            if let CommentTrackState::MaybeBlock(_) | CommentTrackState::InBlockComment(_) =
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
