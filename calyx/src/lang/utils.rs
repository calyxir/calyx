use sexp::Sexp;
use sexp::Sexp::{Atom, List};

// ===============================================
//             Parsing Helper Functions
// ===============================================

/**
 * Converts a Sexp library s-expression to a string
 */
pub fn sexp_to_str(e: &Sexp) -> String {
    match e {
        Atom(sexp::Atom::S(str)) => return String::from(str),
        _ => panic!("Expected str but found: {:?}", e),
    }
}

/**
 * Converts a Sexp library s-expression to an int
 */
pub fn sexp_to_int(e: &Sexp) -> i64 {
    match e {
        Atom(sexp::Atom::I(i)) => return *i,
        _ => panic!("Expected int but found: {:?}", e),
    }
}

/**
 * Grabs the first element in a Sexp List and converts
 * it to a string, if possible. Returns the string and the
 * remaining s-expressions
 */
pub fn get_str(e: &Sexp) -> (String, Sexp) {
    match e {
        Atom(_) => panic!("Expected list with a str head but found: {:?}", e),
        List(vec) => {
            let head = &vec[0];
            let tail = List(vec[1..].to_vec());
            return (sexp_to_str(head), tail);
        }
    }
}

/**
 * Grabs the first element in a Sexp List and converts
 * it to an int, if possible. Returns the int and the
 * remaining s-expressions
 */
pub fn get_int(e: &Sexp) -> (i64, Sexp) {
    match e {
        Atom(_) => panic!("Expected list with an int head but found: {:?}", e),
        List(vec) => {
            let head = &vec[0];
            let tail = List(vec[1..].to_vec());
            return (sexp_to_int(head), tail);
        }
    }
}

/**
 * Unboxes an Sexp into a Vector of S expressions, if it
 * has the proper type.
 */
pub fn get_rest(e: &Sexp) -> Vec<Sexp> {
    match e {
        Atom(_) => panic!("Error: {:?}", e),
        List(vec) => {
            return vec.clone();
        }
    }
}
