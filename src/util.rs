use std::iter::Peekable;
use std::path::PathBuf;

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

pub fn remove_file(path: &mut PathBuf) {
    if path.as_path().file_name().is_some() {
        path.pop();
    }
}