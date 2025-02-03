use calyx_ir::{self as ir};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Default)]
pub struct PromotionAnalysis {
    /// dynamic group Id -> promoted static group Id
    static_group_name: HashMap<ir::Id, ir::Id>,
}

impl PromotionAnalysis {
    fn check_latencies_match(actual: u64, inferred: u64) {
        assert_eq!(actual, inferred, "Inferred and Annotated Latencies do not match. Latency: {}. Inferred: {}", actual, inferred);
    }

    pub fn get_inferred_latency(c: &ir::Control) -> u64 {
        let ir::Control::Static(sc) = c else {
            let Some(latency) = c.get_attribute(ir::NumAttr::Promotable) else {
                unreachable!("Called get_latency on control that is neither static nor promotable")
            };
            return latency;
        };
        sc.get_latency()
    }

    /// Returns true if a control statement is already static, or has the static
    /// attributes
    pub fn can_be_promoted(c: &ir::Control) -> bool {
        c.is_static() || c.has_attribute(ir::NumAttr::Promotable)
    }

    /// If we've already constructed the static group then use the already existing
    /// group. Otherwise construct `static group` and then return that.
    fn construct_static_group(
        &mut self,
        builder: &mut ir::Builder,
        group: ir::RRC<ir::Group>,
        latency: u64,
    ) -> ir::RRC<ir::StaticGroup> {
        if let Some(s_name) = self.static_group_name.get(&group.borrow().name())
        {
            builder.component.find_static_group(*s_name).unwrap()
        } else {
            let sg = builder.add_static_group(group.borrow().name(), latency);
            self.static_group_name
                .insert(group.borrow().name(), sg.borrow().name());
            for assignment in group.borrow().assignments.iter() {
                // Don't need to include assignment to done hole.
                if !(assignment.dst.borrow().is_hole()
                    && assignment.dst.borrow().name == "done")
                {
                    sg.borrow_mut()
                        .assignments
                        .push(ir::Assignment::from(assignment.clone()));
                }
            }
            Rc::clone(&sg)
        }
    }

    // Converts dynamic enable to static
    pub fn convert_enable_to_static(
        &mut self,
        s: &mut ir::Enable,
        builder: &mut ir::Builder,
    ) -> ir::StaticControl {
        s.attributes.remove(ir::NumAttr::Promotable);
        ir::StaticControl::Enable(ir::StaticEnable {
            // upgrading group to static group
            group: self.construct_static_group(
                builder,
                Rc::clone(&s.group),
                s.group
                    .borrow()
                    .get_attributes()
                    .unwrap()
                    .get(ir::NumAttr::Promotable)
                    .unwrap(),
            ),
            attributes: std::mem::take(&mut s.attributes),
        })
    }

    // Converts dynamic invoke to static
    pub fn convert_invoke_to_static(
        &mut self,
        s: &mut ir::Invoke,
    ) -> ir::StaticControl {
        assert!(
            s.comb_group.is_none(),
            "Shouldn't Promote to Static if there is a Comb Group",
        );
        let latency = s.attributes.get(ir::NumAttr::Promotable).unwrap();
        s.attributes.remove(ir::NumAttr::Promotable);
        let s_inv = ir::StaticInvoke {
            comp: Rc::clone(&s.comp),
            inputs: std::mem::take(&mut s.inputs),
            outputs: std::mem::take(&mut s.outputs),
            latency,
            attributes: std::mem::take(&mut s.attributes),
            ref_cells: std::mem::take(&mut s.ref_cells),
            comb_group: std::mem::take(&mut s.comb_group),
        };
        ir::StaticControl::Invoke(s_inv)
    }

    /// Converts control to static control.
    /// Control must already be static or have the `promote_static` attribute.
    pub fn convert_to_static(
        &mut self,
        c: &mut ir::Control,
        builder: &mut ir::Builder,
    ) -> ir::StaticControl {
        assert!(
            c.has_attribute(ir::NumAttr::Promotable) || c.is_static(),
            "Called convert_to_static control that is neither static nor promotable"
        );
        // Need to get bound_attribute here, because we cannot borrow `c` within the
        // pattern match.
        let bound_attribute = c.get_attribute(ir::NumAttr::Bound);
        // Inferred latency of entire control block. Used to double check our
        // function is correct.
        let inferred_latency = Self::get_inferred_latency(c);
        match c {
            ir::Control::Empty(_) => ir::StaticControl::empty(),
            ir::Control::Enable(s) => self.convert_enable_to_static(s, builder),
            ir::Control::Seq(ir::Seq { stmts, attributes }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::Promotable);
                // The resulting static seq should be compactable.
                attributes.insert(ir::NumAttr::Compactable, 1);
                let static_stmts =
                    self.convert_vec_to_static(builder, std::mem::take(stmts));
                let latency =
                    static_stmts.iter().map(|s| s.get_latency()).sum();
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Seq(ir::StaticSeq {
                    stmts: static_stmts,
                    attributes: std::mem::take(attributes),
                    latency,
                })
            }
            ir::Control::Par(ir::Par { stmts, attributes }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::Promotable);
                // Convert stmts to static
                let static_stmts =
                    self.convert_vec_to_static(builder, std::mem::take(stmts));
                // Calculate latency
                let latency = static_stmts
                    .iter()
                    .map(|s| s.get_latency())
                    .max()
                    .unwrap_or_else(|| unreachable!("Empty Par Block"));
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Par(ir::StaticPar {
                    stmts: static_stmts,
                    attributes: ir::Attributes::default(),
                    latency,
                })
            }
            ir::Control::Repeat(ir::Repeat {
                body,
                num_repeats,
                attributes,
            }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::Promotable);
                let sc = self.convert_to_static(body, builder);
                let latency = (*num_repeats) * sc.get_latency();
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Repeat(ir::StaticRepeat {
                    attributes: std::mem::take(attributes),
                    body: Box::new(sc),
                    num_repeats: *num_repeats,
                    latency,
                })
            }
            ir::Control::While(ir::While {
                body, attributes, ..
            }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::Promotable);
                // Removing the `bound` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::Bound);
                let sc = self.convert_to_static(body, builder);
                let num_repeats = bound_attribute.unwrap_or_else(|| unreachable!("Called convert_to_static on a while loop without a bound"));
                let latency = num_repeats * sc.get_latency();
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::Repeat(ir::StaticRepeat {
                    attributes: std::mem::take(attributes),
                    body: Box::new(sc),
                    num_repeats,
                    latency,
                })
            }
            ir::Control::If(ir::If {
                port,
                tbranch,
                fbranch,
                attributes,
                ..
            }) => {
                // Removing the `promote_static` attribute bc we don't need it anymore
                attributes.remove(ir::NumAttr::Promotable);
                let static_tbranch = self.convert_to_static(tbranch, builder);
                let static_fbranch = self.convert_to_static(fbranch, builder);
                let latency = std::cmp::max(
                    static_tbranch.get_latency(),
                    static_fbranch.get_latency(),
                );
                Self::check_latencies_match(latency, inferred_latency);
                ir::StaticControl::static_if(
                    Rc::clone(port),
                    Box::new(static_tbranch),
                    Box::new(static_fbranch),
                    latency,
                )
            }
            ir::Control::Static(_) => c.take_static_control(),
            ir::Control::Invoke(s) => self.convert_invoke_to_static(s),
        }
    }

    /// Converts vec of control to vec of static control.
    /// All control statements in the vec must be promotable or already static.
    pub fn convert_vec_to_static(
        &mut self,
        builder: &mut ir::Builder,
        control_vec: Vec<ir::Control>,
    ) -> Vec<ir::StaticControl> {
        control_vec
            .into_iter()
            .map(|mut c| self.convert_to_static(&mut c, builder))
            .collect()
    }
}
