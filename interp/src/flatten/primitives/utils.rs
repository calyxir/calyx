use ibig::{ibig, UBig};
use ibig::{ubig, IBig};

pub(crate) fn floored_division(left: &IBig, right: &IBig) -> IBig {
    let div = left / right;

    if left.signum() != ibig!(-1) && right.signum() != ibig!(-1) {
        div
    } else if (div.signum() == (-1).into() || div.signum() == 0.into())
        && (left != &(&div * right))
    {
        div - 1_i32
    } else {
        div
    }
}

/// Implementation of integer square root via a basic binary search algorithm
/// based on wikipedia psuedocode
pub(crate) fn int_sqrt(i: &UBig) -> UBig {
    let mut lower: UBig = ubig!(0);
    let mut upper: UBig = i + ubig!(1);
    let mut temp: UBig;

    while lower != (&upper - ubig!(1)) {
        temp = (&lower + &upper) / ubig!(2);
        if &(&temp * &temp) <= i {
            lower = temp
        } else {
            upper = temp
        }
    }
    lower
}
