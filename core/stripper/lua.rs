use super::common::{CommentMatch, End, Start, StripError, find_comments_impl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Normal,
    SawDash1,
    SawDash2,
    LineComment,
    MaybeBlockCommentOpen1,
    MaybeBlockCommentOpen2(usize),
    InBlockComment(usize),
    InBlockCommentSawClose1(usize),
    InBlockCommentSawEquals(usize, usize),
    StringSgl,
    StringSglEsc,
    StringDbl,
    StringDblEsc,
    MaybeLongStringOpen1,
    MaybeLongStringOpen2(usize),
    InLongString(usize),
    InLongStringSawClose1(usize),
    InLongStringSawEquals(usize, usize),
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
    PotentialBlockComment,
    BlockCommentStart(usize),
    BlockCommentEnd,
    ResetPotential,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommentTrackState {
    NotInComment,
    MaybeLine(usize),
    MaybeBlock(usize),
    InLineComment(usize),
    InBlockComment(usize, usize),
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
                '-' => (ParseState::SawDash1, ParseAction::Nothing),
                '[' => (ParseState::MaybeLongStringOpen1, ParseAction::Nothing),
                '\'' => (ParseState::StringSgl, ParseAction::Nothing),
                '"' => (ParseState::StringDbl, ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::Nothing),
            },
            ParseState::SawDash1 => match c {
                '-' => (ParseState::SawDash2, ParseAction::PotentialLineComment),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::SawDash2 => match c {
                '[' => (
                    ParseState::MaybeBlockCommentOpen1,
                    ParseAction::PotentialBlockComment,
                ),
                '\n' => (ParseState::Normal, ParseAction::LineCommentStart),
                _ => (ParseState::LineComment, ParseAction::LineCommentStart),
            },
            ParseState::LineComment => match c {
                '\n' => (ParseState::Normal, ParseAction::LineCommentEnd),
                _ => (ParseState::LineComment, ParseAction::Nothing),
            },

            ParseState::MaybeBlockCommentOpen1 => match c {
                '[' => (
                    ParseState::InBlockComment(0),
                    ParseAction::BlockCommentStart(0),
                ),
                '=' => (ParseState::MaybeBlockCommentOpen2(0), ParseAction::Nothing),
                '\n' => (ParseState::Normal, ParseAction::LineCommentEnd),
                _ => (ParseState::LineComment, ParseAction::Nothing),
            },
            ParseState::MaybeBlockCommentOpen2(level) => match c {
                '=' => (
                    ParseState::MaybeBlockCommentOpen2(level + 1),
                    ParseAction::Nothing,
                ),
                '[' => (
                    ParseState::InBlockComment(level + 1),
                    ParseAction::BlockCommentStart(level + 1),
                ),
                '\n' => (ParseState::Normal, ParseAction::LineCommentEnd),
                _ => (ParseState::LineComment, ParseAction::Nothing),
            },
            ParseState::InBlockComment(level) => match c {
                ']' => (
                    ParseState::InBlockCommentSawClose1(level),
                    ParseAction::Nothing,
                ),
                _ => (ParseState::InBlockComment(level), ParseAction::Nothing),
            },
            ParseState::InBlockCommentSawClose1(level) => match c {
                '=' => (
                    ParseState::InBlockCommentSawEquals(level, 0),
                    ParseAction::Nothing,
                ),
                ']' if level == 0 => (ParseState::Normal, ParseAction::BlockCommentEnd),
                ']' => (
                    ParseState::InBlockCommentSawClose1(level),
                    ParseAction::Nothing,
                ),
                _ => (ParseState::InBlockComment(level), ParseAction::Nothing),
            },
            ParseState::InBlockCommentSawEquals(level, equals_count) => match c {
                '=' => (
                    ParseState::InBlockCommentSawEquals(level, equals_count + 1),
                    ParseAction::Nothing,
                ),
                ']' if level == equals_count + 1 => {
                    (ParseState::Normal, ParseAction::BlockCommentEnd)
                }
                _ => (ParseState::InBlockComment(level), ParseAction::Nothing),
            },

            ParseState::StringSgl => match c {
                '\'' => (ParseState::Normal, ParseAction::Nothing),
                '\\' => (ParseState::StringSglEsc, ParseAction::Nothing),
                _ => (ParseState::StringSgl, ParseAction::Nothing),
            },
            ParseState::StringSglEsc => (ParseState::StringSgl, ParseAction::Nothing),
            ParseState::StringDbl => match c {
                '"' => (ParseState::Normal, ParseAction::Nothing),
                '\\' => (ParseState::StringDblEsc, ParseAction::Nothing),
                _ => (ParseState::StringDbl, ParseAction::Nothing),
            },
            ParseState::StringDblEsc => (ParseState::StringDbl, ParseAction::Nothing),

            ParseState::MaybeLongStringOpen1 => match c {
                '[' => (ParseState::InLongString(0), ParseAction::ResetPotential),
                '=' => (ParseState::MaybeLongStringOpen2(0), ParseAction::Nothing),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::MaybeLongStringOpen2(level) => match c {
                '=' => (
                    ParseState::MaybeLongStringOpen2(level + 1),
                    ParseAction::Nothing,
                ),
                '[' => (
                    ParseState::InLongString(level + 1),
                    ParseAction::ResetPotential,
                ),
                _ => (ParseState::Normal, ParseAction::ResetPotential),
            },
            ParseState::InLongString(level) => match c {
                ']' => (
                    ParseState::InLongStringSawClose1(level),
                    ParseAction::Nothing,
                ),
                _ => (ParseState::InLongString(level), ParseAction::Nothing),
            },
            ParseState::InLongStringSawClose1(level) => match c {
                '=' => (
                    ParseState::InLongStringSawEquals(level, 0),
                    ParseAction::Nothing,
                ),
                ']' if level == 0 => (ParseState::Normal, ParseAction::ResetPotential),
                ']' => (
                    ParseState::InLongStringSawClose1(level),
                    ParseAction::Nothing,
                ),
                _ => (ParseState::InLongString(level), ParseAction::Nothing),
            },
            ParseState::InLongStringSawEquals(level, equals_count) => match c {
                '=' => (
                    ParseState::InLongStringSawEquals(level, equals_count + 1),
                    ParseAction::Nothing,
                ),
                ']' if level == equals_count + 1 => {
                    (ParseState::Normal, ParseAction::ResetPotential)
                }
                _ => (ParseState::InLongString(level), ParseAction::Nothing),
            },

            ParseState::End => (ParseState::End, ParseAction::Nothing),
        },
        None => match from {
            ParseState::SawDash1 => (ParseState::End, ParseAction::ResetPotential),
            ParseState::SawDash2 | ParseState::LineComment => {
                (ParseState::End, ParseAction::LineCommentEnd)
            }
            ParseState::MaybeBlockCommentOpen1
            | ParseState::MaybeBlockCommentOpen2(_)
            | ParseState::InBlockComment(_)
            | ParseState::InBlockCommentSawClose1(_)
            | ParseState::InBlockCommentSawEquals(_, _)
            | ParseState::MaybeLongStringOpen1
            | ParseState::MaybeLongStringOpen2(_)
            | ParseState::InLongString(_)
            | ParseState::InLongStringSawClose1(_)
            | ParseState::InLongStringSawEquals(_, _) => {
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
        ParseAction::PotentialLineComment => {
            if let CommentTrackState::NotInComment = comment_state {
                comment_state = CommentTrackState::MaybeLine(position.saturating_sub(1));
            }
        }
        ParseAction::LineCommentStart => match comment_state {
            CommentTrackState::MaybeLine(from) | CommentTrackState::MaybeBlock(from) => {
                comment_state = CommentTrackState::InLineComment(from);
            }
            CommentTrackState::NotInComment => {
                comment_state = CommentTrackState::InLineComment(position.saturating_sub(1));
            }
            _ => {}
        },
        ParseAction::LineCommentEnd => {
            if let CommentTrackState::InLineComment(from) = comment_state {
                matches.push(CommentMatch { from, to: position });
                comment_state = CommentTrackState::NotInComment;
            } else if let CommentTrackState::MaybeLine(from) = comment_state {
                matches.push(CommentMatch { from, to: position });
                comment_state = CommentTrackState::NotInComment;
            } else if let CommentTrackState::MaybeBlock(from) = comment_state {
                matches.push(CommentMatch { from, to: position });
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::PotentialBlockComment => {
            if let CommentTrackState::NotInComment | CommentTrackState::MaybeLine(_) = comment_state
            {
                comment_state = CommentTrackState::MaybeBlock(position.saturating_sub(2));
            }
        }
        ParseAction::BlockCommentStart(level) => {
            if let CommentTrackState::MaybeBlock(from) = comment_state {
                comment_state = CommentTrackState::InBlockComment(from, level);
            } else {
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::BlockCommentEnd => {
            if let CommentTrackState::InBlockComment(from, _) = comment_state {
                matches.push(CommentMatch {
                    from,
                    to: position + 1,
                });
                comment_state = CommentTrackState::NotInComment;
            }
        }
        ParseAction::ResetPotential => {
            if let CommentTrackState::MaybeLine(_)
            | CommentTrackState::MaybeBlock(_)
            | CommentTrackState::InBlockComment(_, _) = comment_state
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
