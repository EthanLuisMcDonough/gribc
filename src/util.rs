use std::iter::Peekable;

pub fn next_if<J, I: Iterator<Item = J>>(
    iter: &mut Peekable<I>,
    predicate: impl Fn(&J) -> bool,
) -> Option<J> {
    if iter.peek().filter(|e| predicate(e)).is_some() {
        iter.next()
    } else {
        None
    }
}
