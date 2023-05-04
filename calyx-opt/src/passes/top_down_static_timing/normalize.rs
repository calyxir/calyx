use calyx_ir::{self as ir, GetAttributes};
use std::{cmp, iter};

/// Implements normalization for static `if` and `while`.
/// - `if`: Ensure both the branches take the same number of cycles.
/// - `while`: Directly nested bounded loops are de-nested.
pub struct Normalize {
    /// Enable for the group used to balance `if` branches.
    balance: ir::Enable,
}

impl Normalize {
    /// Apply the `if` and `while` normalization to a static control program.
    /// **Requires**: The control program has a static attribute.
    pub fn apply(con: &mut ir::Control, builder: &mut ir::Builder) {
        debug_assert!(
            con.get_attribute(ir::NumAttr::Static).is_some(),
            "Attempting to normalize non-static program"
        );
        let balance = builder.add_group("balance");
        balance
            .borrow_mut()
            .attributes
            .insert(ir::NumAttr::Static, 1);
        let mut balance = ir::Enable {
            group: balance,
            attributes: ir::Attributes::default(),
        };
        balance.attributes.insert(ir::NumAttr::Static, 1);
        let norm = Normalize { balance };
        norm.recur(con);
    }

    fn recur(&self, con: &mut ir::Control) {
        match con {
            ir::Control::Par(ir::Par { stmts, .. })
            | ir::Control::Seq(ir::Seq { stmts, .. }) => {
                stmts.iter_mut().for_each(|c| self.recur(c))
            }
            ir::Control::If(ir::If {
                tbranch, fbranch, ..
            }) => {
                self.recur(tbranch);
                self.recur(fbranch);
                let ttime = tbranch.get_attribute(ir::NumAttr::Static).unwrap();
                let ftime = fbranch.get_attribute(ir::NumAttr::Static).unwrap();
                let max_time = cmp::max(ttime, ftime);
                self.extend_control(tbranch, max_time, &self.balance);
                self.extend_control(fbranch, max_time, &self.balance);
            }
            ir::Control::While(wh) => {
                Self::denest_loop(wh);
                self.recur(&mut wh.body);
            }
            ir::Control::Invoke(_)
            | ir::Control::Enable(_)
            | ir::Control::Static(_)
            | ir::Control::Empty(_) => {}
        }
    }

    /// Take a control program and extend it to ensure that its execution time is at least `time`.
    /// **Requires**: The control program must have a `static` attribute.
    fn extend_control(
        &self,
        con: &mut Box<ir::Control>,
        time: u64,
        balance: &ir::Enable,
    ) {
        let cur_time = con.get_attribute(ir::NumAttr::Static).unwrap();

        if cur_time < time {
            let bal = ir::Control::Enable(ir::Cloner::enable(balance));
            let inner = *std::mem::replace(con, Box::new(ir::Control::empty()));
            let extra = (0..time - cur_time).map(|_| ir::Cloner::control(&bal));
            let mut seq = if matches!(&inner, ir::Control::Empty(_)) {
                ir::Control::seq(extra.collect())
            } else {
                ir::Control::seq(iter::once(inner).chain(extra).collect())
            };
            seq.get_mut_attributes().insert(ir::NumAttr::Static, time);
            *con = Box::new(seq);
        }
    }

    /// Transform nested bounded loops into a singly nested loop:
    /// ```
    /// @bound(m) while r0.out {
    ///   @bound(n) while r1.out {
    ///     @bound(l) while r2.out { body }
    ///   }
    /// }
    /// ```
    /// Into:
    /// ```
    /// @bound(m*n*l) while r0.out { body }
    /// ```
    ///
    /// Note that after this transformation, it is no longer correct to lower
    /// the loop using TDCC since we've ignored the loop entry conditions.
    fn denest_loop(wh: &mut ir::While) {
        let mut body =
            std::mem::replace(&mut wh.body, Box::new(ir::Control::empty()));
        let mut bound = wh.attributes.get(ir::NumAttr::Bound).unwrap();
        let mut body_time = body.get_attribute(ir::NumAttr::Static).unwrap();

        while let ir::Control::While(inner) = *body {
            bound *= inner.attributes.get(ir::NumAttr::Bound).unwrap();
            body = inner.body;
            body_time = body.get_attribute(ir::NumAttr::Static).unwrap();
        }
        wh.body = body;
        wh.attributes.insert(ir::NumAttr::Bound, bound);
        wh.attributes.insert(ir::NumAttr::Static, body_time * bound);
    }
}
