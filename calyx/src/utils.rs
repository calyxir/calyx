/**
 * Combine concatenates [vec] into a single string, with each entry
 * separated by [delimiter], and [end] appended to the end result
 */
pub fn combine(vec: &Vec<String>, delimiter: &str, end: &str) -> String {
    let mut s = String::new();
    let n = vec.len() - 1;
    for i in 0..n {
        s.push_str(vec[i].as_ref());
        s.push_str(delimiter);
    }
    s.push_str(vec[n].as_ref());
    s.push_str(end);
    return s;
}