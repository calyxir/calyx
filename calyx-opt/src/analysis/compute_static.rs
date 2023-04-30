use calyx_ir::{self as ir, GetAttributes};
use std::collections::HashMap;

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
                .insert(ir::Attribute::Static, time);
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
            ir::Control::Invoke(inv) => inv.update_static(extra),
            ir::Control::Enable(en) => en.update_static(&()),
            ir::Control::Empty(_) => Some(0),
            ir::Control::Static(sc) => Some(sc.get_latency()),
        }
    }
}

impl WithStatic for ir::StaticEnable {
    type Info = ();
    fn compute_static(&mut self, _: &Self::Info) -> Option<u64> {
        // Attempt to get the latency from the attribute on the enable first, or
        // failing that, from the group.
        Some(self.group.borrow().get_latency())
    }
}

impl WithStatic for ir::Enable {
    type Info = ();
    fn compute_static(&mut self, _: &Self::Info) -> Option<u64> {
        // Attempt to get the latency from the attribute on the enable first, or
        // failing that, from the group.
        self.attributes.get(ir::Attribute::Static).or_else(|| {
            self.group.borrow().attributes.get(ir::Attribute::Static)
        })
    }
}

impl WithStatic for ir::Invoke {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        self.attributes.get(ir::Attribute::Static).or_else(|| {
            let comp = self.comp.borrow().type_name()?;
            extra.get(&comp).cloned()
        })
    }
}

impl WithStatic for ir::Seq {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        let mut sum = 0;
        for stmt in &mut self.stmts {
            sum += stmt.update_static(extra)?;
        }
        Some(sum)
    }
}

impl WithStatic for ir::Par {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        let mut max = 0;
        for stmt in &mut self.stmts {
            max = std::cmp::max(max, stmt.update_static(extra)?);
        }
        Some(max)
    }
}

impl WithStatic for ir::If {
    type Info = CompTime;
    fn compute_static(&mut self, extra: &Self::Info) -> Option<u64> {
        let t = self.tbranch.update_static(extra)?;
        let f = self.fbranch.update_static(extra)?;
        // Cannot compute latency information for `if`-`with`
        if self.cond.is_some() {
            log::debug!("Cannot compute latency for while-with");
            return None;
        }
        Some(std::cmp::max(t, f))
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
        let bound = self.attributes.get(ir::Attribute::Bound)?;
        Some(bound * b_time)
    }
}
