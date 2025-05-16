pub type StripError = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentMatch {
    pub from: usize,
    pub to: usize,
}

pub trait Start: Sized {
    fn start() -> Self;
}

pub trait End: Sized {
    fn end() -> Self;
}

pub fn find_comments_impl<ParseState, ParseAction, CommentState, StateTransitionFn, DoActionFn>(
    input: &str,
    state_transition: StateTransitionFn,
    do_action: DoActionFn,
) -> Result<Vec<CommentMatch>, StripError>
where
    ParseState: Start + End + Copy + Eq,
    ParseAction: Copy + Eq,
    CommentState: Start + Copy + Eq,
    StateTransitionFn: Fn(ParseState, Option<char>) -> (ParseState, ParseAction),
    DoActionFn: Fn(
        ParseAction,
        CommentState,
        usize,
        Vec<CommentMatch>,
    ) -> Result<(CommentState, Vec<CommentMatch>), StripError>,
{
    let mut matches = Vec::new();
    let mut current_parse_state = ParseState::start();
    let mut current_comment_state = CommentState::start();
    let mut char_indices = input.char_indices().peekable();

    loop {
        let char_info = char_indices.next();
        let current_char = char_info.map(|(_, c)| c);
        let position = char_info.map_or(input.len(), |(idx, _)| idx);

        let (next_parse_state, action) = state_transition(current_parse_state, current_char);

        let (next_comment_state, next_matches) =
            do_action(action, current_comment_state, position, matches)?;

        current_parse_state = next_parse_state;
        current_comment_state = next_comment_state;
        matches = next_matches;

        if current_char.is_none() {
            if current_parse_state != ParseState::end() {
                let (final_state, final_action) = state_transition(current_parse_state, None);
                if final_state == ParseState::end() || final_action != action {
                    let (_, final_matches) =
                        do_action(final_action, current_comment_state, input.len(), matches)?;
                    matches = final_matches;
                }
            }
            break;
        }
    }
    Ok(matches)
}

pub fn remove_matches(
    mut input: String,
    mut matches: Vec<CommentMatch>,
) -> Result<String, StripError> {
    if matches.is_empty() {
        return Ok(input);
    }
    check_matches_bounds(&input, &matches)?;

    matches.sort_by_key(|m| m.from);
    check_sorted_matches_overlap(&matches)?;
    matches.reverse();

    for m in matches {
        if m.from <= input.len() && m.to <= input.len() && m.from <= m.to {
            input.drain(m.from..m.to);
        } else {
            return Err("Invalid match range encountered during removal");
        }
    }
    Ok(input)
}

fn check_matches_bounds(input: &str, matches: &[CommentMatch]) -> Result<(), StripError> {
    let len = input.len();
    for m in matches {
        if m.from > len || m.to > len || m.from > m.to {
            eprintln!("Invalid bounds: from={}, to={}, len={}", m.from, m.to, len);
            return Err("Match indices out of bounds or invalid range (from > to)");
        }
    }
    Ok(())
}

fn check_sorted_matches_overlap(matches: &[CommentMatch]) -> Result<(), StripError> {
    let mut last_to = 0;
    for m in matches {
        if m.from < last_to {
            eprintln!("Overlap: from={}, last_to={}", m.from, last_to);
            return Err("Matches are overlapping");
        }
        last_to = m.to;
    }
    Ok(())
}
