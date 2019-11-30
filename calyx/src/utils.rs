/**
 * Combine concatenates [vec] into a single string, with each entry
 * separated by [delimiter], and [end] appended to the end result
 */
pub fn combine(vec: &[String], delimiter: &str, end: &str) -> String {
    if vec.is_empty() {
        "".to_string()
    } else {
        let mut s = String::new();
        let n = vec.len() - 1;
        for x in vec.iter().take(n) {
            s.push_str(x);
            s.push_str(delimiter);
        }
        s.push_str(vec[n].as_ref());
        s.push_str(end);
        s
    }
}
