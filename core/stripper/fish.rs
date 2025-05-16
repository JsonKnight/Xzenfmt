use super::common::{CommentMatch, StripError, find_comments_impl};
use super::shell::{
    CommentTrackState as ShCommentState, ParseAction as ShParseAction, ShParseState,
    do_action as sh_do_action, sh_state_transition,
};

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    find_comments_impl::<ShParseState, ShParseAction, ShCommentState, _, _>(
        input,
        sh_state_transition,
        sh_do_action,
    )
}
