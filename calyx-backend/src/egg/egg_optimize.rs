//! Calyx => Egglog -> (optimize) => Calyx

use super::utils::RewriteRule;
use super::utils::{self};
use crate::egg::calyx_to_egg;
use crate::traits::Backend;
use calyx_ir::{self as ir};
use ir::Component;
use itertools::Itertools;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::{fs, io};
use strum::IntoEnumIterator;

#[derive(Default)]
pub struct EggOptimizeBackend;

impl Backend for EggOptimizeBackend {
    fn name(&self) -> &'static str {
        "egg-optimize"
    }

    fn validate(_prog: &ir::Context) -> calyx_utils::CalyxResult<()> {
        Ok(())
    }

    fn emit(
        ctx: &ir::Context,
        file: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        if ctx.components.len() > 1 {
            todo!("multiple components not supported in CalyxEgg")
        }
        let component: &Component = ctx
            .components
            .first()
            .unwrap_or_else(|| panic!("no components found"));

        // Convert this Calyx program to its egglog equivalent.
        let rules = utils::RewriteRule::iter().collect_vec();

        let mut f1 = tempfile::NamedTempFile::new()?;
        writeln!(f1, "{}", Self::egglog_rules(&rules)?)?;
        calyx_to_egg::ToEggPrinter::write_component(component, &mut f1)?;
        writeln!(f1)?;
        writeln!(f1, "{}", Self::egglog_schedule(&rules)?)?;

        let mut buf = String::new();
        f1.flush()?;
        f1.seek(io::SeekFrom::Start(0))?;
        f1.read_to_string(&mut buf)?;

        // Now, parse and extract the optimized Calyx control schedule, and write the component back to our file.
        let (term, termdag) = utils::extract_egglog(
            ctx.bc.display_egraph,
            component.name.id.into(),
            &buf,
        );

        utils::convert_component(
            component,
            term,
            &termdag,
            &mut file.get_write(),
        )
        .unwrap_or_else(|_| {
            panic!("failed to convert component: {:?}", component)
        });
        Ok(())
    }

    fn link_externs(
        _prog: &ir::Context,
        _write: &mut calyx_utils::OutputFile,
    ) -> calyx_utils::CalyxResult<()> {
        Ok(())
    }
}

impl EggOptimizeBackend {
    /// Retrieve the datatypes and rewrite rules for this Calyx-egglog program.
    pub fn egglog_rules(
        rules: &[RewriteRule],
    ) -> calyx_utils::CalyxResult<String> {
        let mut s = String::new();
        // Half-baked: path used for `fud`.
        let path = Path::new("calyx-backend/src/egg/ruleset/");
        for rule in rules {
            s.push_str(&fs::read_to_string(
                path.join(rule.to_string()).with_extension("egg"),
            )?);
        }
        Ok(s)
    }

    /// Retrieves the schedule for this Calyx-egglog program.
    pub fn egglog_schedule(
        _rules: &[RewriteRule],
    ) -> calyx_utils::CalyxResult<String> {
        // TODO(cgyurgyik): This was chosen with little care.
        Ok(r#"
        (run-schedule
            (saturate analysis)
            (repeat 32 
                (saturate 
                    fan-out 
                    par-to-seq
                    split-seq
                    static-compaction
                    analysis
                    collapse-control
                ) 
                (run)
            )
        )"#
        .to_string())
    }
}
