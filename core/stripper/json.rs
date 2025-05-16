use super::c_family;
use super::common::{CommentMatch, StripError};

pub fn find_comments(input: &str) -> Result<Vec<CommentMatch>, StripError> {
    c_family::find_comments(input)
}
