use calyx_ir::{self as ir, GetAttributes};
use std::collections::HashMap;
use std::rc::Rc;

/// Trait to propagate and extra "static" attributes through [ir::Control].
/// Calling the update function ensures that the current program, as well as all
/// sub-programs have a "static" attribute on them.
/// Usage:
/// ```
/// use calyx::analysis::compute_static::WithStatic;
/// let con: ir::Control = todo!(); // A complex control program
/// con.update(&HashMap::new());    // Compute the static information for the program
/// ```
pub trait WithStatic
where
    Self: GetAttributes,
{
    /// Extra information needed to compute static information for this type.
    type Info;

    /// Compute the static information for the type if possible and add it to its attribute.
    /// Implementors should instead implement [WithStatic::compute_static] and call this function
    /// on sub-programs.
    /// **Ensures**: All sub-programs of the type will also be updated.
    fn update_static(&mut self, extra: &Self::Info) -> Option<u64> {
        if let Some(time) = self.compute_static(extra) {
            self.get_mut_attributes()
                .insert(ir::NumAttr::Promotable, time);
            Some(time)
        } else {
            None
        }
    }

    /// Compute the static information for the type if possible and update all sub-programs.
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64>;
}

type CompTime = HashMap<ir::Id, u64>;

impl WithStatic for ir::Control {
    // Mapping from name of components to their latency information
    type Info = CompTime;

    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        match self {
            ir::Control::Seq(seq) => seq.update_static(extra),
            ir::Control::Par(par) => par.update_static(extra),
            ir::Control::If(if_) => if_.update_static(extra),
            ir::Control::While(wh) => wh.update_static(extra),
            ir::Control::Repeat(rep) => rep.update_static(extra),
            ir::Control::Invoke(inv) => inv.update_static(extra),
            ir::Control::Enable(en) => en.update_static(&()),
            ir::Control::Empty(_) => Some(0),
            ir::Control::Static(sc) => Some(sc.get_latency()),
        }
    }
}

impl WithStatic for ir::Enable {
    type Info = ();
    fn compute_static(&mut self, _: &Self::Info) -> Option<u64> {
        // Attempt to get the latency from the attribute on the enable first, or
        // failing that, from the group.
        self.attributes.get(ir::NumAttr::Promotable).or_else(|| {
            self.group.borrow().attributes.get(ir::NumAttr::Promotable)
        })
    }
}

impl WithStatic for ir::Invoke {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        self.attributes.get(ir::NumAttr::Promotable).or_else(|| {
            let comp = self.comp.borrow().type_name()?;
            extra.get(&comp).cloned()
        })
    }
}

/// Walk over a set of control statements and call `update_static` on each of them.
/// Use a merge function to merge the results of the `update_static` calls.
fn walk_static<T, F>(stmts: &mut [T], extra: &T::Info, merge: F) -> Option<u64>
where
    T: WithStatic,
    F: Fn(u64, u64) -> u64,
{
    let mut latency = Some(0);
    // This is implemented as a loop because we want to call `update_static` on
    // each statement even if we cannot compute a total latency anymore.
    for stmt in stmts.iter_mut() {
        let stmt_latency = stmt.update_static(extra);
        latency = match (latency, stmt_latency) {
            (Some(l), Some(s)) => Some(merge(l, s)),
            (_, _) => None,
        }
    }
    latency
}

impl WithStatic for ir::Seq {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        walk_static(&mut self.stmts, extra, |x, y| x + y)
    }
}

impl WithStatic for ir::Par {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        walk_static(&mut self.stmts, extra, std::cmp::max)
    }
}

impl WithStatic for ir::If {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        // Cannot compute latency information for `if`-`with`
        let t_latency = self.tbranch.update_static(extra);
        let f_latency = self.fbranch.update_static(extra);
        if self.cond.is_some() {
            log::debug!("Cannot compute latency for while-with");
            return None;
        }
        match (t_latency, f_latency) {
            (Some(t), Some(f)) => Some(std::cmp::max(t, f)),
            (_, _) => None,
        }
    }
}

impl WithStatic for ir::While {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        let b_time = self.body.update_static(extra)?;
        // Cannot compute latency information for `while`-`with`
        if self.cond.is_some() {
            log::debug!("Cannot compute latency for while-with");
            return None;
        }
        let bound = self.attributes.get(ir::NumAttr::Bound)?;
        Some(bound * b_time)
    }
}

impl WithStatic for ir::Repeat {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        let b_time = self.body.update_static(extra)?;
        let num_repeats = self.num_repeats;
        Some(num_repeats * b_time)
    }
}

pub trait IntoStatic {
    type StaticCon;
    fn make_static(&mut self) -> Option<Self::StaticCon>;
}

impl IntoStatic for ir::Seq {
    type StaticCon = ir::StaticSeq;
    fn make_static(&mut self) -> Option<Self::StaticCon> {
        let mut static_stmts: Vec<ir::StaticControl> = Vec::new();
        let mut latency = 0;
        for stmt in self.stmts.iter() {
            if !matches!(stmt, ir::Control::Static(_)) {
                log::debug!("Cannot build `static seq`. Control statement inside `seq` is not static");
                return None;
            }
        }

        for stmt in self.stmts.drain(..) {
            let ir::Control::Static(sc) = stmt else {
                unreachable!("We have already checked that all control statements are static")
            };
            latency += sc.get_latency();
            static_stmts.push(sc);
        }
        Some(ir::StaticSeq {
            stmts: static_stmts,
            attributes: self.attributes.clone(),
            latency,
        })
    }
}

impl IntoStatic for ir::Par {
    type StaticCon = ir::StaticPar;
    fn make_static(&mut self) -> Option<Self::StaticCon> {
        let mut static_stmts: Vec<ir::StaticControl> = Vec::new();
        let mut latency = 0;
        for stmt in self.stmts.iter() {
            if !matches!(stmt, ir::Control::Static(_)) {
                log::debug!("Cannot build `static seq`. Control statement inside `seq` is not static");
                return None;
            }
        }

        for stmt in self.stmts.drain(..) {
            let ir::Control::Static(sc) = stmt else {
                unreachable!("We have already checked that all control statements are static")
            };
            latency = std::cmp::max(latency, sc.get_latency());
            static_stmts.push(sc);
        }
        Some(ir::StaticPar {
            stmts: static_stmts,
            attributes: self.attributes.clone(),
            latency,
        })
    }
}

impl IntoStatic for ir::If {
    type StaticCon = ir::StaticIf;
    fn make_static(&mut self) -> Option<Self::StaticCon> {
        if !(self.tbranch.is_static() && self.fbranch.is_static()) {
            return None;
        };
        let tb = std::mem::replace(&mut *self.tbranch, ir::Control::empty());
        let fb = std::mem::replace(&mut *self.fbranch, ir::Control::empty());
        let ir::Control::Static(sc_t) = tb else {
            unreachable!("we have already checked tbranch to be static")
        };
        let ir::Control::Static(sc_f) = fb else {
            unreachable!("we have already checker fbranch to be static")
        };
        let latency = std::cmp::max(sc_t.get_latency(), sc_f.get_latency());
        Some(ir::StaticIf {
            tbranch: Box::new(sc_t),
            fbranch: Box::new(sc_f),
            attributes: ir::Attributes::default(),
            port: Rc::clone(&self.port),
            latency,
        })
    }
}
